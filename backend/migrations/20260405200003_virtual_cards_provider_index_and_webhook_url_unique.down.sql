ALTER TABLE webhook_endpoints DROP CONSTRAINT IF EXISTS uk_webhook_endpoints_url;

DROP INDEX IF EXISTS idx_cards_provider;
