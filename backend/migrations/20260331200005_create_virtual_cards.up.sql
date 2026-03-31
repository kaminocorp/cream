-- Phase 3, Migration 5: virtual_cards
CREATE TABLE virtual_cards (
    id               UUID PRIMARY KEY,
    agent_id         UUID NOT NULL REFERENCES agents(id),
    provider_id      TEXT NOT NULL,
    provider_card_id TEXT NOT NULL,
    card_type        TEXT NOT NULL CHECK (card_type IN ('single_use', 'multi_use')),
    controls         JSONB NOT NULL,
    status           TEXT NOT NULL DEFAULT 'active'
                     CHECK (status IN ('active', 'frozen', 'cancelled', 'expired')),
    created_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at       TIMESTAMPTZ
);

CREATE INDEX idx_cards_agent ON virtual_cards(agent_id);
CREATE INDEX idx_cards_status ON virtual_cards(status);
