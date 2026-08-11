use serde_json::{Value, json};

use super::*;

fn succeeded_job(intent: &str, profile: JobPolicyProfile) -> Value {
    let (
        provider,
        deployment,
        placement,
        policy,
        provider_access,
        requested_placement,
        prefer,
        offline,
        latency,
    ) = match profile {
        JobPolicyProfile::ShapeLocalInteractive => (
            "mlx-audio",
            "qwen-local",
            "local",
            "local-first",
            Value::Null,
            "local_only",
            "local",
            true,
            json!("interactive"),
        ),
        JobPolicyProfile::ShapeCloudImageInteractive => (
            "codex-subscription",
            "codex_gpt_5_6_luna",
            "cloud",
            "balanced",
            json!("subscription"),
            "cloud_only",
            "cloud",
            false,
            Value::Null,
        ),
    };
    json!({
        "id": "resp_shape_1",
        "app_id": "shape",
        "intent": intent,
        "provider": provider,
        "deployment": deployment,
        "model_profile": deployment,
        "model_build": format!("{deployment}_build"),
        "physical_model": "physical-model",
        "placement": placement,
        "capability_level": "capable",
        "evaluation_status": "provisional",
        "resource_class": "standard",
        "state": "succeeded",
        "policy": policy,
        "priority": "interactive",
        "constraints": {
            "policy": policy,
            "priority": "interactive",
            "provider_access_class": provider_access,
            "placement": requested_placement,
            "prefer": prefer,
            "offline_required": offline,
            "latency": latency,
            "fallback": "none",
            "max_cost_usd": 0.0,
            "deadline_ms": null,
            "capability_floor": "capable"
        },
        "routing": {
            "capability_floor": "capable",
            "candidates": [{
                "provider": provider,
                "deployment": deployment,
                "status": "eligible",
                "rank": 1,
                "reason_codes": []
            }]
        },
        "attempts": [{
            "number": 1,
            "provider": provider,
            "deployment": deployment,
            "outcome": "succeeded",
            "trigger": "initial",
            "error_kind": null
        }],
        "error": null
    })
}

#[test]
fn cloud_image_profile_preserves_exact_requested_and_actual_provenance() {
    let bytes = serde_json::to_vec(&succeeded_job(
        "image.generate",
        JobPolicyProfile::ShapeCloudImageInteractive,
    ))
    .unwrap();
    let provenance = parse_job_snapshot(
        InferRuntimeContractRevision::Candidate3,
        "resp_shape_1",
        "image.generate",
        JobPolicyProfile::ShapeCloudImageInteractive,
        &bytes,
    )
    .unwrap();
    assert_eq!(provenance.provider, "codex-subscription");
    assert_eq!(provenance.placement, "cloud");
    assert_eq!(provenance.requested_policy, "balanced");
    assert_eq!(
        provenance.requested_provider_access_class.as_deref(),
        Some("subscription")
    );
    assert!(!provenance.offline_required);
    assert_eq!(provenance.attempts.len(), 1);
}

#[test]
fn mismatched_intent_route_attempt_and_policy_fail_closed() {
    let base = succeeded_job(
        "image.generate",
        JobPolicyProfile::ShapeCloudImageInteractive,
    );
    for mutation in [
        ("intent", json!("language.respond")),
        ("placement", json!("local")),
    ] {
        let mut job = base.clone();
        job[mutation.0] = mutation.1;
        assert!(
            parse_job_snapshot(
                InferRuntimeContractRevision::Candidate3,
                "resp_shape_1",
                "image.generate",
                JobPolicyProfile::ShapeCloudImageInteractive,
                &serde_json::to_vec(&job).unwrap(),
            )
            .is_err()
        );
    }

    let mut fallback = base.clone();
    fallback["attempts"][0]["trigger"] = json!("fallback");
    assert_eq!(
        parse_job_snapshot(
            InferRuntimeContractRevision::Candidate3,
            "resp_shape_1",
            "image.generate",
            JobPolicyProfile::ShapeCloudImageInteractive,
            &serde_json::to_vec(&fallback).unwrap(),
        ),
        Err(JobProvenanceError::PolicyViolation)
    );

    let mut mismatch = base;
    mismatch["attempts"][0]["deployment"] = json!("other-deployment");
    assert_eq!(
        parse_job_snapshot(
            InferRuntimeContractRevision::Candidate3,
            "resp_shape_1",
            "image.generate",
            JobPolicyProfile::ShapeCloudImageInteractive,
            &serde_json::to_vec(&mismatch).unwrap(),
        ),
        Err(JobProvenanceError::InvalidResponse)
    );
}

#[test]
fn job_identity_is_bounded_to_a_response_id_path_segment() {
    assert!(valid_job_id("resp_shape_image_1"));
    assert!(valid_job_id("job_shape_speech_1"));
    assert!(!valid_job_id("resp_../operator"));
    assert!(!valid_job_id("resp_query?elsewhere"));
}
