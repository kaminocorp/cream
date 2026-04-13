-- Revert rule_type renames in built-in templates.
UPDATE policy_templates
SET rules = (
    SELECT jsonb_agg(
        CASE
            WHEN elem->>'rule_type' = 'category_check'
            THEN jsonb_set(elem, '{rule_type}', '"category_restriction"')
            WHEN elem->>'rule_type' = 'geographic'
            THEN jsonb_set(elem, '{rule_type}', '"geographic_restriction"')
            ELSE elem
        END
        ORDER BY ordinality
    )
    FROM jsonb_array_elements(rules) WITH ORDINALITY AS t(elem, ordinality)
)
WHERE is_builtin = true;
