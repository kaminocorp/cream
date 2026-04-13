-- Provider API key storage with AES-256-GCM encryption at rest.
-- Keys are encrypted before INSERT; only the last 4 chars are stored in plaintext for display.
CREATE TABLE provider_api_keys (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    provider_name TEXT NOT NULL UNIQUE
        CHECK (provider_name IN ('stripe', 'airwallex', 'coinbase')),
    encrypted_key BYTEA NOT NULL,
    key_preview   TEXT NOT NULL,  -- last 4 chars for masked display
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TRIGGER set_updated_at BEFORE UPDATE ON provider_api_keys
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
