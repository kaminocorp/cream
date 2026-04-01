DROP TRIGGER IF EXISTS agent_profiles_updated_at ON agent_profiles;
DROP TABLE IF EXISTS agent_profiles;
-- Safe to drop here: this down migration runs LAST (reverse chronological
-- order), so all tables with triggers referencing set_updated_at() are
-- already dropped by the time this executes.
DROP FUNCTION IF EXISTS set_updated_at();
