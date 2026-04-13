use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::error::ApiError;
use crate::extractors::auth::AuthenticatedOperator;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// AES-256-GCM helpers
// ---------------------------------------------------------------------------

fn encrypt_key(plaintext: &[u8], secret: &[u8]) -> Result<Vec<u8>, ApiError> {
    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce};

    let cipher = Aes256Gcm::new_from_slice(secret)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("AES key init: {e}")))?;

    // Generate random 12-byte nonce.
    let nonce_bytes: [u8; 12] = rand::random();
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("AES encrypt: {e}")))?;

    // Prepend nonce to ciphertext for storage.
    let mut result = nonce_bytes.to_vec();
    result.extend_from_slice(&ciphertext);
    Ok(result)
}

/// Decrypt a provider API key. Used when dispatching payments to providers
/// (Phase 17+). Exposed here alongside `encrypt_key` so both are tested together.
#[allow(dead_code)]
fn decrypt_key(encrypted: &[u8], secret: &[u8]) -> Result<Vec<u8>, ApiError> {
    use aes_gcm::aead::Aead;
    use aes_gcm::{Aes256Gcm, KeyInit, Nonce};

    if encrypted.len() < 13 {
        return Err(ApiError::Internal(anyhow::anyhow!(
            "encrypted data too short"
        )));
    }

    let (nonce_bytes, ciphertext) = encrypted.split_at(12);
    let cipher = Aes256Gcm::new_from_slice(secret)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("AES key init: {e}")))?;
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| ApiError::Internal(anyhow::anyhow!("AES decrypt: {e}")))
}

fn get_encryption_secret(state: &AppState) -> Result<&[u8], ApiError> {
    state
        .config
        .provider_key_encryption_secret
        .as_deref()
        .ok_or_else(|| {
            ApiError::Internal(anyhow::anyhow!(
                "PROVIDER_KEY_ENCRYPTION_SECRET not configured — provider key storage unavailable"
            ))
        })
}

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct SaveProviderKeyRequest {
    pub provider_name: String,
    pub api_key: String,
}

#[derive(Debug, Serialize)]
pub struct ProviderKeyInfo {
    pub id: String,
    pub provider_name: String,
    pub key_preview: String,
    pub created_at: String,
    pub updated_at: String,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// `PUT /v1/settings/provider-keys` — save (upsert) an encrypted provider API key.
pub async fn save_provider_key(
    State(state): State<AppState>,
    _op: AuthenticatedOperator,
    Json(body): Json<SaveProviderKeyRequest>,
) -> Result<(StatusCode, Json<ProviderKeyInfo>), ApiError> {
    // Validate provider name.
    let valid_providers = ["stripe", "airwallex", "coinbase"];
    if !valid_providers.contains(&body.provider_name.as_str()) {
        return Err(ApiError::ValidationError(format!(
            "invalid provider_name '{}', must be one of: {}",
            body.provider_name,
            valid_providers.join(", ")
        )));
    }
    if body.api_key.trim().is_empty() {
        return Err(ApiError::ValidationError(
            "api_key must not be empty".to_string(),
        ));
    }

    let secret = get_encryption_secret(&state)?;
    let encrypted = encrypt_key(body.api_key.as_bytes(), secret)?;

    // Last 4 chars for masked preview.
    let key_preview = if body.api_key.len() >= 4 {
        format!("...{}", &body.api_key[body.api_key.len() - 4..])
    } else {
        "****".to_string()
    };

    // Upsert via ON CONFLICT.
    let row = sqlx::query_as::<_, (uuid::Uuid, String, String, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>(
        r#"
        INSERT INTO provider_api_keys (provider_name, encrypted_key, key_preview)
        VALUES ($1, $2, $3)
        ON CONFLICT (provider_name)
        DO UPDATE SET encrypted_key = EXCLUDED.encrypted_key,
                      key_preview = EXCLUDED.key_preview,
                      updated_at = now()
        RETURNING id, provider_name, key_preview, created_at, updated_at
        "#,
    )
    .bind(&body.provider_name)
    .bind(&encrypted)
    .bind(&key_preview)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("save provider key: {e}")))?;

    Ok((
        StatusCode::OK,
        Json(ProviderKeyInfo {
            id: row.0.to_string(),
            provider_name: row.1,
            key_preview: row.2,
            created_at: row.3.to_rfc3339(),
            updated_at: row.4.to_rfc3339(),
        }),
    ))
}

/// `GET /v1/settings/provider-keys` — list provider keys (masked, last 4 chars only).
pub async fn list_provider_keys(
    State(state): State<AppState>,
    _op: AuthenticatedOperator,
) -> Result<Json<Vec<ProviderKeyInfo>>, ApiError> {
    let rows = sqlx::query_as::<_, (uuid::Uuid, String, String, chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>(
        r#"
        SELECT id, provider_name, key_preview, created_at, updated_at
        FROM provider_api_keys
        ORDER BY provider_name
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::Internal(anyhow::anyhow!("list provider keys: {e}")))?;

    let keys: Vec<ProviderKeyInfo> = rows
        .into_iter()
        .map(|row| ProviderKeyInfo {
            id: row.0.to_string(),
            provider_name: row.1,
            key_preview: row.2,
            created_at: row.3.to_rfc3339(),
            updated_at: row.4.to_rfc3339(),
        })
        .collect();

    Ok(Json(keys))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_round_trip() {
        let secret = [0xABu8; 32]; // 32 bytes = AES-256
        let plaintext = b"sk_live_1234567890abcdef";

        let encrypted = encrypt_key(plaintext, &secret).unwrap();
        let decrypted = decrypt_key(&encrypted, &secret).unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn encrypt_produces_different_ciphertexts_for_same_input() {
        let secret = [0xCDu8; 32];
        let plaintext = b"same_key";

        let enc1 = encrypt_key(plaintext, &secret).unwrap();
        let enc2 = encrypt_key(plaintext, &secret).unwrap();

        // Different nonces → different ciphertexts.
        assert_ne!(enc1, enc2);

        // Both decrypt to the same plaintext.
        assert_eq!(decrypt_key(&enc1, &secret).unwrap(), plaintext);
        assert_eq!(decrypt_key(&enc2, &secret).unwrap(), plaintext);
    }

    #[test]
    fn decrypt_with_wrong_secret_fails() {
        let secret1 = [0xAAu8; 32];
        let secret2 = [0xBBu8; 32];
        let plaintext = b"sk_live_secret";

        let encrypted = encrypt_key(plaintext, &secret1).unwrap();
        let result = decrypt_key(&encrypted, &secret2);

        assert!(result.is_err());
    }

    #[test]
    fn key_preview_shows_last_4_chars() {
        let key = "sk_live_1234567890abcdef";
        let preview = if key.len() >= 4 {
            format!("...{}", &key[key.len() - 4..])
        } else {
            "****".to_string()
        };
        assert_eq!(preview, "...cdef");
    }

    #[test]
    fn key_preview_short_key_masked() {
        let key = "abc";
        let preview = if key.len() >= 4 {
            format!("...{}", &key[key.len() - 4..])
        } else {
            "****".to_string()
        };
        assert_eq!(preview, "****");
    }

    #[test]
    fn decrypt_too_short_data_fails() {
        let secret = [0xAAu8; 32];
        let result = decrypt_key(&[1, 2, 3], &secret);
        assert!(result.is_err());
    }
}
