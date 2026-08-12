use serde_json::{Value, json};

use super::*;

struct ProfileValues {
    provider: &'static str,
    deployment: &'static str,
    model_profile: &'static str,
    placement: &'static str,
    policy: &'static str,
    provider_access: Value,
    requested_placement: &'static str,
    prefer: &'static str,
    offline: bool,
    latency: Value,
    capability_floor: &'static str,
    named_route: Value,
}

fn profile_values(profile: JobPolicyProfile) -> ProfileValues {
    match profile {
        JobPolicyProfile::LocalInteractive => ProfileValues {
            provider: "mlx-audio",
            deployment: "qwen-local",
            model_profile: "qwen-local",
            placement: "local",
            policy: "local-first",
            provider_access: Value::Null,
            requested_placement: "local_only",
            prefer: "local",
            offline: true,
            latency: json!("interactive"),
            capability_floor: "capable",
            named_route: Value::Null,
        },
        JobPolicyProfile::LocalTextEdit => ProfileValues {
            provider: "ollama-local",
            deployment: "ollama_qwen3_5_4b",
            model_profile: "qwen3_5_4b",
            placement: "local",
            policy: "local-first",
            provider_access: Value::Null,
            requested_placement: "local_only",
            prefer: "local",
            offline: true,
            latency: Value::Null,
            capability_floor: "foundational",
            named_route: json!({
                "kind": "deployment",
                "ordered_ids": ["ollama_qwen3_5_4b"]
            }),
        },
        JobPolicyProfile::CloudImageInteractive => ProfileValues {
            provider: "codex-subscription",
            deployment: "codex_gpt_5_6_luna",
            model_profile: "codex_gpt_5_6_luna",
            placement: "cloud",
            policy: "balanced",
            provider_access: json!("subscription"),
            requested_placement: "cloud_only",
            prefer: "cloud",
            offline: false,
            latency: Value::Null,
            capability_floor: "capable",
            named_route: Value::Null,
        },
    }
}

fn succeeded_job(intent: &str, profile: JobPolicyProfile) -> Value {
    let values = profile_values(profile);
    json!({
        "id": "resp_shape_1",
        "app_id": "shape",
        "intent": intent,
        "provider": values.provider,
        "deployment": values.deployment,
        "model_profile": values.model_profile,
        "model_build": format!("{}_build", values.deployment),
        "physical_model": "physical-model",
        "placement": values.placement,
        "capability_level": values.capability_floor,
        "evaluation_status": "provisional",
        "resource_class": "standard",
        "state": "succeeded",
        "policy": values.policy,
        "priority": "interactive",
        "constraints": {
            "policy": values.policy,
            "priority": "interactive",
            "provider_access_class": values.provider_access,
            "placement": values.requested_placement,
            "prefer": values.prefer,
            "offline_required": values.offline,
            "latency": values.latency,
            "fallback": "none",
            "max_cost_usd": 0.0,
            "deadline_ms": null,
            "capability_floor": values.capability_floor,
            "named_route": values.named_route.clone()
        },
        "routing": {
            "capability_floor": values.capability_floor,
            "named_route": values.named_route,
            "candidates": [{
                "provider": values.provider,
                "deployment": values.deployment,
                "status": "eligible",
                "rank": 1,
                "reason_codes": []
            }]
        },
        "attempts": [{
            "number": 1,
            "provider": values.provider,
            "deployment": values.deployment,
            "outcome": "succeeded",
            "trigger": "initial",
            "error_kind": null
        }],
        "error": null
    })
}

#[test]
fn current_text_edit_requires_the_exact_authorized_named_route() {
    let job = succeeded_job("text.edit", JobPolicyProfile::LocalTextEdit);
    let provenance = parse_job_snapshot(
        "resp_shape_1",
        "text.edit",
        JobPolicyProfile::LocalTextEdit,
        &serde_json::to_vec(&job).unwrap(),
    )
    .expect("current named text route validates");
    assert_eq!(provenance.capability_floor, "foundational");
    assert_eq!(provenance.deployment, "ollama_qwen3_5_4b");
    assert_eq!(
        provenance.named_route,
        Some(ExternalNamedRouteProvenance {
            kind: "deployment".to_owned(),
            ordered_ids: vec!["ollama_qwen3_5_4b".to_owned()],
        })
    );

    let mut wrong_route = job;
    wrong_route["constraints"]["named_route"]["ordered_ids"] = json!(["other"]);
    assert_eq!(
        parse_job_snapshot(
            "resp_shape_1",
            "text.edit",
            JobPolicyProfile::LocalTextEdit,
            &serde_json::to_vec(&wrong_route).unwrap(),
        ),
        Err(JobProvenanceError::InvalidResponse)
    );
}

#[test]
fn cloud_image_profile_preserves_exact_requested_and_actual_provenance() {
    let bytes = serde_json::to_vec(&succeeded_job(
        "image.generate",
        JobPolicyProfile::CloudImageInteractive,
    ))
    .unwrap();
    let provenance = parse_job_snapshot(
        "resp_shape_1",
        "image.generate",
        JobPolicyProfile::CloudImageInteractive,
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
    let base = succeeded_job("image.generate", JobPolicyProfile::CloudImageInteractive);
    for mutation in [
        ("intent", json!("wrong.intent")),
        ("placement", json!("local")),
    ] {
        let mut job = base.clone();
        job[mutation.0] = mutation.1;
        assert!(
            parse_job_snapshot(
                "resp_shape_1",
                "image.generate",
                JobPolicyProfile::CloudImageInteractive,
                &serde_json::to_vec(&job).unwrap(),
            )
            .is_err()
        );
    }

    let mut fallback = base.clone();
    fallback["attempts"][0]["trigger"] = json!("fallback");
    assert_eq!(
        parse_job_snapshot(
            "resp_shape_1",
            "image.generate",
            JobPolicyProfile::CloudImageInteractive,
            &serde_json::to_vec(&fallback).unwrap(),
        ),
        Err(JobProvenanceError::PolicyViolation)
    );

    let mut mismatch = base;
    mismatch["attempts"][0]["deployment"] = json!("other-deployment");
    assert_eq!(
        parse_job_snapshot(
            "resp_shape_1",
            "image.generate",
            JobPolicyProfile::CloudImageInteractive,
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
