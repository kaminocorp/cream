-- v0.8.9: Ensure existing policy_rules.action values match the uppercase CHECK
-- constraint added in migration 20260405200008. Without this, any database with
-- pre-existing lowercase action values ('approve', 'block', 'escalate') would
-- fail the CHECK constraint validation. This is a no-op on fresh databases.
UPDATE policy_rules SET action = UPPER(action) WHERE action != UPPER(action);
