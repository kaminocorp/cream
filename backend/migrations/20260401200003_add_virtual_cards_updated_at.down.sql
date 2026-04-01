DROP TRIGGER IF EXISTS virtual_cards_updated_at ON virtual_cards;
ALTER TABLE virtual_cards DROP COLUMN IF EXISTS updated_at;
