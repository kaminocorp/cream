//! Async audit export endpoints (Phase 17-E).
//!
//! - `POST /v1/audit/export` — create an export job
//! - `GET /v1/audit/exports/{id}` — poll export status

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::audit_export::{
    self, ExportDestination, ExportFilters, ExportFormat,
};
use crate::error::ApiError;
use crate::extractors::auth::AuthenticatedOperator;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateExportRequest {
    pub filters: Option<ExportFilters>,
    /// Export format: `csv` or `ndjson`.
    pub format: String,
    pub destination: ExportDestination,
}

#[derive(Debug, Serialize)]
pub struct CreateExportResponse {
    pub export_id: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ExportStatusResponse {
    pub export_id: String,
    pub status: String,
    pub format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows_exported: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `POST /v1/audit/export` — create an async export job. Operator-only.
pub async fn create_export(
    State(state): State<AppState>,
    _op: AuthenticatedOperator,
    Json(body): Json<CreateExportRequest>,
) -> Result<(StatusCode, Json<CreateExportResponse>), ApiError> {
    let format = ExportFormat::from_str_loose(&body.format).ok_or_else(|| {
        ApiError::ValidationError(format!(
            "unsupported export format '{}'; use 'csv' or 'ndjson'",
            body.format
        ))
    })?;

    if body.destination.dest_type != "s3" {
        return Err(ApiError::ValidationError(format!(
            "unsupported destination type '{}'; only 's3' is supported",
            body.destination.dest_type
        )));
    }

    // Enforce concurrency cap on active export jobs.
    let active = audit_export::count_active_exports(&state.db)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("check active exports: {e}")))?;
    if active >= audit_export::MAX_CONCURRENT_EXPORTS {
        return Err(ApiError::ValidationError(format!(
            "too many active export jobs ({active}); wait for existing exports to complete before starting a new one"
        )));
    }

    // Verify S3 is configured.
    if state.config.audit_export_s3_bucket.is_none() && body.destination.bucket.is_none() {
        return Err(ApiError::Internal(anyhow::anyhow!(
            "S3 audit export not configured — set AUDIT_EXPORT_S3_BUCKET or provide bucket in request"
        )));
    }

    let filters = body.filters.unwrap_or(ExportFilters {
        from: None,
        to: None,
        agent_id: None,
        status: None,
        category: None,
    });

    let job_id = audit_export::create_export_job(&state.db, format, &filters, &body.destination)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("create export job: {e}")))?;

    // Spawn background task.
    let db = state.db.clone();
    let reader = state.audit_reader.clone();
    let config = state.config.clone();
    tokio::spawn(async move {
        audit_export::execute_export(db, reader, config, job_id).await;
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(CreateExportResponse {
            export_id: job_id.to_string(),
            status: "pending".to_string(),
        }),
    ))
}

/// `GET /v1/audit/exports/{id}` — poll export status. Operator-only.
pub async fn get_export_status(
    State(state): State<AppState>,
    _op: AuthenticatedOperator,
    Path(id): Path<String>,
) -> Result<Json<ExportStatusResponse>, ApiError> {
    let job_id = id
        .parse::<uuid::Uuid>()
        .map_err(|e| ApiError::ValidationError(format!("invalid export ID: {e}")))?;

    let job = audit_export::get_export_job(&state.db, job_id)
        .await
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("get export job: {e}")))?
        .ok_or_else(|| ApiError::NotFound(format!("export {id}")))?;

    Ok(Json(ExportStatusResponse {
        export_id: job.id.to_string(),
        status: job.status,
        format: job.format,
        rows_exported: job.rows_exported,
        s3_key: job.s3_key,
        error_message: job.error_message,
        created_at: job.created_at.to_rfc3339(),
        completed_at: job.completed_at.map(|t| t.to_rfc3339()),
    }))
}
