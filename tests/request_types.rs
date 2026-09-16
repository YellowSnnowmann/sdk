use serde_json::json;
use tinyhumans_sdk::api::types::{
    BillingPlan, CreateFeedbackRequest, FeedbackType, PurchasePlanRequest,
};

#[test]
fn documented_enums_use_backend_wire_values() {
    let feedback = CreateFeedbackRequest {
        kind: FeedbackType::Feature,
        title: "Typed SDK".into(),
        body: "Expose request models".into(),
    };
    assert_eq!(
        serde_json::to_value(feedback).unwrap(),
        json!({
            "type": "feature",
            "title": "Typed SDK",
            "body": "Expose request models"
        })
    );

    let purchase = PurchasePlanRequest {
        plan: BillingPlan::ProYearly,
        success_url: Some("https://example.test/success".into()),
        cancel_url: None,
    };
    assert_eq!(
        serde_json::to_value(purchase).unwrap(),
        json!({
            "plan": "PRO_YEARLY",
            "successUrl": "https://example.test/success"
        })
    );
}
