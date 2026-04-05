-- Reverse Phase 7.10: settlement field pairing constraint

ALTER TABLE payments
    DROP CONSTRAINT IF EXISTS chk_payments_settlement_pair;
