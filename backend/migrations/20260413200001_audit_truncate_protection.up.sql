-- Protect the append-only audit_log against TRUNCATE.
--
-- The existing row-level triggers (audit_log_no_update, audit_log_no_delete)
-- prevent UPDATE and DELETE on individual rows, but TRUNCATE bypasses
-- row-level triggers entirely in PostgreSQL. This statement-level trigger
-- closes that gap, ensuring the immutable ledger cannot be wiped.
CREATE OR REPLACE FUNCTION prevent_audit_truncate()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'TRUNCATE on audit_log is prohibited — append-only ledger';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER audit_log_no_truncate
    BEFORE TRUNCATE ON audit_log
    FOR EACH STATEMENT EXECUTE FUNCTION prevent_audit_truncate();
