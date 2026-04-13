DROP TRIGGER IF EXISTS audit_log_no_truncate ON audit_log;
DROP FUNCTION IF EXISTS prevent_audit_truncate();
