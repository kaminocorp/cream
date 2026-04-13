DROP TABLE IF EXISTS webhook_delivery_log;

-- Remove agent_id column and its index from webhook_endpoints.
DROP INDEX IF EXISTS idx_webhook_endpoints_agent_id;
ALTER TABLE webhook_endpoints DROP COLUMN IF EXISTS agent_id;
