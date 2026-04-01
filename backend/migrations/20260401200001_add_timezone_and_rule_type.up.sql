-- Add timezone column to agent_profiles (supports TimeWindowEvaluator timezone-aware checks)
ALTER TABLE agent_profiles ADD COLUMN timezone TEXT;

-- Add rule_type column to policy_rules (explicit type avoids fragile inference from condition fields)
ALTER TABLE policy_rules ADD COLUMN rule_type TEXT;
