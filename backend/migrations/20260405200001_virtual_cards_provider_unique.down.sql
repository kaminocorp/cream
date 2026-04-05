-- Reverse Phase 7.9: virtual_cards composite unique constraint

ALTER TABLE virtual_cards
    DROP CONSTRAINT IF EXISTS uk_virtual_cards_provider_card;
