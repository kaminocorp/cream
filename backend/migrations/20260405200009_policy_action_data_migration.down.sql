-- Reverse: convert uppercase actions back to lowercase for the old CHECK constraint.
UPDATE policy_rules SET action = LOWER(action) WHERE action != LOWER(action);
