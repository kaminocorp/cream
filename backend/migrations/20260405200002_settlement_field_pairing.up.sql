-- Phase 7.10: Enforce amount_settled / settled_currency pairing
--
-- If amount_settled is set, settled_currency must also be set (and vice
-- versa). A payment with a settled amount but no currency makes
-- reconciliation impossible. Additive constraint — does not modify
-- existing constraints.

ALTER TABLE payments
    ADD CONSTRAINT chk_payments_settlement_pair
        CHECK (
            (amount_settled IS NULL AND settled_currency IS NULL)
            OR (amount_settled IS NOT NULL AND settled_currency IS NOT NULL)
        );
