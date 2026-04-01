DROP TRIGGER IF EXISTS webhook_endpoints_updated_at ON webhook_endpoints;
ALTER TABLE webhook_endpoints DROP COLUMN IF EXISTS updated_at;
