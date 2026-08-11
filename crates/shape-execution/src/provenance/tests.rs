use serde_json::{Value, json};

use super::ExternalExecutionProvenance;

#[test]
fn historical_candidate_two_fields_deserialize_but_serialize_with_current_names() {
    let historical = json!({
        "contract_revision": "0.1.0-candidate.2",
        "app_id": "shape",
        "intent": "speech.synthesize",
        "provider": "mlx-audio-local",
        "deployment": "speech-local",
        "model_profile": "speech-profile",
        "model_build": "speech-build",
        "physical_model": "speech-model",
        "placement": "local",
        "quality_grade": "general",
        "rating_status": "provisional",
        "resource_class": "standard",
        "policy": "local-first",
        "priority": "interactive",
        "requested_policy": "local-first",
        "requested_priority": "interactive",
        "requested_provider_access_class": null,
        "requested_placement": "local_only",
        "requested_preference": "local",
        "offline_required": true,
        "requested_latency": "interactive",
        "fallback": "none",
        "requested_deadline_ms": null,
        "max_cost_microusd": 0,
        "quality_floor": "general",
        "routing_candidates": [{
            "provider": "mlx-audio-local",
            "deployment": "speech-local",
            "status": "eligible",
            "rank": 1,
            "reason_codes": []
        }],
        "attempts": [{
            "number": 1,
            "provider": "mlx-audio-local",
            "deployment": "speech-local",
            "outcome": "succeeded",
            "trigger": "initial",
            "error_kind": null
        }]
    });

    let provenance: ExternalExecutionProvenance =
        serde_json::from_value(historical).expect("historical provenance remains readable");
    assert_eq!(provenance.capability_level, "general");
    assert_eq!(provenance.evaluation_status, "provisional");
    assert_eq!(provenance.capability_floor, "general");
    assert!(provenance.is_bounded());

    let current = serde_json::to_value(provenance).expect("current provenance serializes");
    let object = current.as_object().expect("provenance is an object");
    assert_eq!(current["capability_level"], Value::String("general".into()));
    assert_eq!(
        current["evaluation_status"],
        Value::String("provisional".into())
    );
    assert_eq!(current["capability_floor"], Value::String("general".into()));
    assert!(!object.contains_key("quality_grade"));
    assert!(!object.contains_key("rating_status"));
    assert!(!object.contains_key("quality_floor"));
}
