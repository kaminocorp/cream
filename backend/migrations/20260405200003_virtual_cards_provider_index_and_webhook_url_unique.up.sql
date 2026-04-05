-- Index on virtual_cards(provider_id) for provider-level card lookups.
-- The existing composite unique (provider_id, provider_card_id) does not serve
-- as a leading index for provider_id-only queries.
CREATE INDEX idx_cards_provider ON virtual_cards(provider_id);

-- Prevent duplicate webhook endpoint registrations at the DB level.
ALTER TABLE webhook_endpoints
    ADD CONSTRAINT uk_webhook_endpoints_url UNIQUE (url);
