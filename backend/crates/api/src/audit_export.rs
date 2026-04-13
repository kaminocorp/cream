//! Async audit export engine (Phase 17-E).
//!
//! Streams audit rows in chunks, formats to CSV or NDJSON, and uploads to S3.
//! Export jobs are tracked in the `audit_exports` table and can be polled for
//! status via `GET /v1/audit/exports/{id}`.

use std::sync::Arc;

use chrono::Utc;
use cream_audit::{AuditQuery, AuditReader};
use cream_models::prelude::*;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::config::AppConfig;

/// Chunk size for streaming audit rows during export.
const EXPORT_CHUNK_SIZE: u32 = 1_000;

/// Flatten an AuditEntry into a vector of string fields for CSV export.
/// Column order: entry_id, timestamp, agent_id, payment_id, amount, currency,
/// status, decision, provider, justification_summary.
pub fn flatten_entry(entry: &AuditEntry) -> Vec<String> {
    let amount = entry
        .request
        .get("amount")
        .map(|v| {
            if let Some(s) = v.as_str() {
                s.to_string()
            } else {
                v.to_string()
            }
        })
        .unwrap_or_default();

    let currency = entry
        .request
        .get("currency")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let provider = entry
        .provider_response
        .as_ref()
        .map(|p| p.provider.to_string())
        .unwrap_or_default();

    let justification_summary = entry
        .justification
        .get("summary")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    vec![
        entry.id.to_string(),
        entry.timestamp.to_rfc3339(),
        entry.agent_id.to_string(),
        entry.payment_id.map(|p| p.to_string()).unwrap_or_default(),
        amount,
        currency,
        format!("{:?}", entry.final_status),
        format!("{:?}", entry.policy_evaluation.final_decision),
        provider,
        justification_summary,
    ]
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportFilters {
    pub from: Option<String>,
    pub to: Option<String>,
    pub agent_id: Option<String>,
    pub status: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportDestination {
    #[serde(rename = "type")]
    pub dest_type: String,
    /// S3 bucket override (optional — falls back to config).
    pub bucket: Option<String>,
    /// S3 key prefix override.
    pub key_prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportJob {
    pub id: uuid::Uuid,
    pub status: String,
    pub format: String,
    pub filters: serde_json::Value,
    pub destination: serde_json::Value,
    pub rows_exported: Option<i64>,
    pub s3_key: Option<String>,
    pub error_message: Option<String>,
    pub created_at: chrono::DateTime<Utc>,
    pub completed_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Csv,
    Ndjson,
}

impl ExportFormat {
    pub fn from_str_loose(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "csv" => Some(Self::Csv),
            "ndjson" => Some(Self::Ndjson),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Ndjson => "ndjson",
        }
    }
}

// ---------------------------------------------------------------------------
// Job creation
// ---------------------------------------------------------------------------

/// Create a new export job record and return its ID.
pub async fn create_export_job(
    db: &PgPool,
    format: ExportFormat,
    filters: &ExportFilters,
    destination: &ExportDestination,
) -> Result<uuid::Uuid, sqlx::Error> {
    let id = uuid::Uuid::now_v7();
    let filters_json = serde_json::to_value(filters).unwrap_or_default();
    let dest_json = serde_json::to_value(destination).unwrap_or_default();

    sqlx::query(
        "INSERT INTO audit_exports (id, format, filters, destination)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(format.as_str())
    .bind(&filters_json)
    .bind(&dest_json)
    .execute(db)
    .await?;

    Ok(id)
}

/// Get an export job by ID.
pub async fn get_export_job(db: &PgPool, id: uuid::Uuid) -> Result<Option<ExportJob>, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: uuid::Uuid,
        status: String,
        format: String,
        filters: serde_json::Value,
        destination: serde_json::Value,
        rows_exported: Option<i64>,
        s3_key: Option<String>,
        error_message: Option<String>,
        created_at: chrono::DateTime<Utc>,
        completed_at: Option<chrono::DateTime<Utc>>,
    }

    let row: Option<Row> = sqlx::query_as(
        "SELECT id, status, format, filters, destination, rows_exported,
                s3_key, error_message, created_at, completed_at
         FROM audit_exports WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(db)
    .await?;

    Ok(row.map(|r| ExportJob {
        id: r.id,
        status: r.status,
        format: r.format,
        filters: r.filters,
        destination: r.destination,
        rows_exported: r.rows_exported,
        s3_key: r.s3_key,
        error_message: r.error_message,
        created_at: r.created_at,
        completed_at: r.completed_at,
    }))
}

// ---------------------------------------------------------------------------
// Export execution (background task)
// ---------------------------------------------------------------------------

/// Run an export job: stream audit rows, format, upload to S3.
pub async fn execute_export(
    db: PgPool,
    audit_reader: Arc<dyn AuditReader>,
    config: Arc<AppConfig>,
    job_id: uuid::Uuid,
) {
    // Mark as running.
    if let Err(e) = sqlx::query("UPDATE audit_exports SET status = 'running' WHERE id = $1")
        .bind(job_id)
        .execute(&db)
        .await
    {
        tracing::error!(job_id = %job_id, error = %e, "failed to mark export as running");
        return;
    }

    match run_export_inner(&db, &audit_reader, &config, job_id).await {
        Ok((rows, s3_key)) => {
            let _ = sqlx::query(
                "UPDATE audit_exports SET status = 'completed', rows_exported = $1, s3_key = $2, completed_at = now()
                 WHERE id = $3",
            )
            .bind(rows)
            .bind(&s3_key)
            .bind(job_id)
            .execute(&db)
            .await;

            tracing::info!(job_id = %job_id, rows, s3_key = %s3_key, "audit export completed");
        }
        Err(e) => {
            let msg = format!("{e:#}");
            let _ = sqlx::query(
                "UPDATE audit_exports SET status = 'failed', error_message = $1, completed_at = now()
                 WHERE id = $2",
            )
            .bind(&msg)
            .bind(job_id)
            .execute(&db)
            .await;

            tracing::error!(job_id = %job_id, error = %msg, "audit export failed");
        }
    }
}

async fn run_export_inner(
    db: &PgPool,
    audit_reader: &Arc<dyn AuditReader>,
    config: &Arc<AppConfig>,
    job_id: uuid::Uuid,
) -> Result<(i64, String), anyhow::Error> {
    // Load job details.
    let job = get_export_job(db, job_id)
        .await?
        .ok_or_else(|| anyhow::anyhow!("export job {job_id} not found"))?;

    let format = ExportFormat::from_str_loose(&job.format)
        .ok_or_else(|| anyhow::anyhow!("unknown export format: {}", job.format))?;

    let filters: ExportFilters = serde_json::from_value(job.filters)?;
    let destination: ExportDestination = serde_json::from_value(job.destination)?;

    // Resolve S3 bucket.
    let bucket = destination
        .bucket
        .or_else(|| config.audit_export_s3_bucket.clone())
        .ok_or_else(|| {
            anyhow::anyhow!("no S3 bucket configured — set AUDIT_EXPORT_S3_BUCKET or provide bucket in destination")
        })?;

    let region = config
        .audit_export_s3_region
        .as_deref()
        .unwrap_or("us-east-1");

    let key_prefix = destination
        .key_prefix
        .or_else(|| config.audit_export_s3_prefix.clone())
        .unwrap_or_default();

    let extension = match format {
        ExportFormat::Csv => "csv",
        ExportFormat::Ndjson => "ndjson",
    };
    let s3_key = format!(
        "{}export_{}_{}.{extension}",
        key_prefix,
        job_id,
        Utc::now().format("%Y%m%dT%H%M%SZ"),
    );

    // Stream chunks and accumulate output.
    let mut all_rows: Vec<AuditEntry> = Vec::new();
    let mut offset: u32 = 0;

    loop {
        let mut q = AuditQuery::new()
            .limit(EXPORT_CHUNK_SIZE)
            .offset(offset);

        if let Some(ref from) = filters.from {
            if let Ok(ts) = from.parse::<chrono::DateTime<chrono::Utc>>() {
                q = q.from(ts);
            }
        }
        if let Some(ref to) = filters.to {
            if let Ok(ts) = to.parse::<chrono::DateTime<chrono::Utc>>() {
                q = q.to(ts);
            }
        }
        if let Some(ref agent_id) = filters.agent_id {
            if let Ok(id) = agent_id.parse::<AgentId>() {
                q = q.agent_id(id);
            }
        }
        if let Some(ref status) = filters.status {
            if let Ok(s) = serde_json::from_value::<PaymentStatus>(serde_json::json!(status)) {
                q = q.status(s);
            }
        }
        if let Some(ref category) = filters.category {
            if let Ok(c) = serde_json::from_value::<PaymentCategory>(serde_json::json!(category)) {
                q = q.category(c);
            }
        }

        let chunk = audit_reader.query(q).await?;
        let chunk_len = chunk.len() as u32;
        all_rows.extend(chunk);
        offset += chunk_len;

        if chunk_len < EXPORT_CHUNK_SIZE {
            break;
        }
    }

    let total_rows = all_rows.len() as i64;

    // Format the output.
    let body = match format {
        ExportFormat::Csv => {
            let mut writer = csv::Writer::from_writer(Vec::new());
            writer.write_record([
                "entry_id", "timestamp", "agent_id", "payment_id", "amount",
                "currency", "status", "decision", "provider", "justification_summary",
            ])?;
            for entry in &all_rows {
                let row = flatten_entry(entry);
                writer.write_record(&row)?;
            }
            writer.into_inner()?
        }
        ExportFormat::Ndjson => {
            let mut buf = Vec::new();
            for entry in &all_rows {
                if let Ok(line) = serde_json::to_vec(entry) {
                    buf.extend_from_slice(&line);
                    buf.push(b'\n');
                }
            }
            buf
        }
    };

    // Upload to S3.
    let aws_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_sdk_s3::config::Region::new(region.to_string()))
        .load()
        .await;
    let s3_client = aws_sdk_s3::Client::new(&aws_config);

    let content_type = match format {
        ExportFormat::Csv => "text/csv",
        ExportFormat::Ndjson => "application/x-ndjson",
    };

    s3_client
        .put_object()
        .bucket(&bucket)
        .key(&s3_key)
        .body(body.into())
        .content_type(content_type)
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("S3 upload failed: {e}"))?;

    Ok((total_rows, s3_key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn export_format_from_str_loose() {
        assert_eq!(ExportFormat::from_str_loose("csv"), Some(ExportFormat::Csv));
        assert_eq!(ExportFormat::from_str_loose("CSV"), Some(ExportFormat::Csv));
        assert_eq!(ExportFormat::from_str_loose("ndjson"), Some(ExportFormat::Ndjson));
        assert_eq!(ExportFormat::from_str_loose("NDJSON"), Some(ExportFormat::Ndjson));
        assert_eq!(ExportFormat::from_str_loose("xml"), None);
    }

    #[test]
    fn export_format_round_trips() {
        assert_eq!(
            ExportFormat::from_str_loose(ExportFormat::Csv.as_str()),
            Some(ExportFormat::Csv)
        );
        assert_eq!(
            ExportFormat::from_str_loose(ExportFormat::Ndjson.as_str()),
            Some(ExportFormat::Ndjson)
        );
    }
}
