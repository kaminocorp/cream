-- Phase 16-B: Operator authentication — real per-user identity replacing
-- the interim shared OPERATOR_API_KEY.

CREATE TABLE operators (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email         TEXT NOT NULL UNIQUE,
    name          TEXT NOT NULL
        CHECK (char_length(TRIM(name)) BETWEEN 1 AND 200),
    password_hash TEXT NOT NULL,
    role          TEXT NOT NULL DEFAULT 'admin'
        CHECK (role IN ('admin', 'viewer')),
    status        TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'suspended')),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_login_at TIMESTAMPTZ
);

CREATE TRIGGER set_updated_at BEFORE UPDATE ON operators
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
CREATE INDEX idx_operators_email ON operators(email);
