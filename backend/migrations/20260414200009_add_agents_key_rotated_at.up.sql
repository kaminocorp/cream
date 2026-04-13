-- Phase 17-D: Track when agent API keys were last rotated.
-- Backfill with created_at so existing agents don't immediately trigger
-- the credential age monitor.
ALTER TABLE agents ADD COLUMN key_rotated_at TIMESTAMPTZ;
UPDATE agents SET key_rotated_at = created_at;
ALTER TABLE agents ALTER COLUMN key_rotated_at SET NOT NULL;
ALTER TABLE agents ALTER COLUMN key_rotated_at SET DEFAULT now();
