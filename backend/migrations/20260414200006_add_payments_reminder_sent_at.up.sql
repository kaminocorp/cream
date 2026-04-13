-- Phase 16-H: Track when the escalation reminder was sent so it is not re-sent
-- on every monitor tick. NULL = no reminder sent yet.
ALTER TABLE payments ADD COLUMN reminder_sent_at TIMESTAMPTZ;
