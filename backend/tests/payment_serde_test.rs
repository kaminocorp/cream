use cream_models::prelude::*;
use rust_decimal::Decimal;

#[test]
fn payment_request_serde_roundtrip() {
    let req = PaymentRequest {
        agent_id: AgentId::new(),
        amount: Decimal::new(10000, 2),
        currency: Currency::USD,
        recipient: Recipient {
            recipient_type: RecipientType::Merchant,
            identifier: "test".to_string(),
            name: None,
            country: None,
        },
        preferred_rail: RailPreference::Auto,
        justification: Justification {
            summary: "Test payment".to_string(),
            task_id: None,
            category: PaymentCategory::ApiCredits,
            expected_value: None,
        },
        metadata: None,
        idempotency_key: IdempotencyKey::new("test-key"),
    };
    
    let json = serde_json::to_string(&req).unwrap();
    let parsed: PaymentRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(req.agent_id, parsed.agent_id);
    println!("PaymentRequest serde OK");
}

#[test]
fn payment_serde_roundtrip() {
    let req = PaymentRequest {
        agent_id: AgentId::new(),
        amount: Decimal::new(10000, 2),
        currency: Currency::USD,
        recipient: Recipient {
            recipient_type: RecipientType::Merchant,
            identifier: "test".to_string(),
            name: None,
            country: None,
        },
        preferred_rail: RailPreference::Auto,
        justification: Justification {
            summary: "Test payment".to_string(),
            task_id: None,
            category: PaymentCategory::ApiCredits,
            expected_value: None,
        },
        metadata: None,
        idempotency_key: IdempotencyKey::new("test-key"),
    };
    
    let p = Payment::new(req);
    let json = serde_json::to_string(&p).unwrap();
    println!("Serialized: {}", json);
    let parsed: Payment = serde_json::from_str(&json).unwrap();
    assert_eq!(p.id, parsed.id);
    println!("Payment serde OK");
}
