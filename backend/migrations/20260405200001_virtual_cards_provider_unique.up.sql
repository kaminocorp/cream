-- Phase 7.9: composite unique constraint on virtual_cards(provider_id, provider_card_id)
--
-- Prevents duplicate card IDs from the same provider being silently stored.
-- Additive migration. Does not modify existing constraints or indexes.

ALTER TABLE virtual_cards
    ADD CONSTRAINT uk_virtual_cards_provider_card
    UNIQUE (provider_id, provider_card_id);
