-- Rust enforces MAX_IDEMPOTENCY_KEY_LEN = 255 on deserialization, but the DB
-- allowed unbounded TEXT. Aligns with the defense-in-depth pattern from
-- v0.8.4-v0.8.6 (name, provider_id, provider_tx_id, on_chain_tx_hash).
ALTER TABLE idempotency_keys
    ADD CONSTRAINT chk_idempotency_key_length
        CHECK (LENGTH(key) <= 255 AND LENGTH(TRIM(key)) > 0);
