-- Fix built-in template seed data: rule_type values must match the 12
-- registered policy engine evaluators in VALID_RULE_TYPES.
--
-- - category_restriction → category_check
-- - geographic_restriction → geographic

UPDATE policy_templates
SET rules = (
    SELECT jsonb_agg(
        CASE
            WHEN elem->>'rule_type' = 'category_restriction'
            THEN jsonb_set(elem, '{rule_type}', '"category_check"')
            WHEN elem->>'rule_type' = 'geographic_restriction'
            THEN jsonb_set(elem, '{rule_type}', '"geographic"')
            ELSE elem
        END
        ORDER BY ordinality
    )
    FROM jsonb_array_elements(rules) WITH ORDINALITY AS t(elem, ordinality)
)
WHERE is_builtin = true
  AND rules::text LIKE '%_restriction%';
