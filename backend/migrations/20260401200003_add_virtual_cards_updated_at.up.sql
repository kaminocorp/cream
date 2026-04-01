-- Phase 6.3: Add missing updated_at column + trigger to virtual_cards
-- Every other mutable table has this pattern; virtual_cards was the sole omission.

ALTER TABLE virtual_cards
    ADD COLUMN updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

CREATE TRIGGER virtual_cards_updated_at
    BEFORE UPDATE ON virtual_cards
    FOR EACH ROW EXECUTE FUNCTION set_updated_at();
