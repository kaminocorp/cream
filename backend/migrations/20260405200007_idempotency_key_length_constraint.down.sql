ALTER TABLE idempotency_keys
    DROP CONSTRAINT IF EXISTS chk_idempotency_key_length;
