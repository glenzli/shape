use infer_runtime_client::{
    AttemptSnapshot, CandidateDecision, JobSnapshot, NamedRouteDecision, RoutingDecision,
};
use serde_json::json;

use super::{
    JobPolicyProfile, JobProvenanceError, SPEECH_DEPLOYMENT, TEXT_EDIT_DEPLOYMENT,
    parse_job_snapshot,
};
use crate::infer_runtime::{
    INFER_RUNTIME_CONTRACT_VERSION, INFER_RUNTIME_RESPONSES_CAPABILITY,
    INFER_RUNTIME_SPEECH_CAPABILITY,
};

fn local_job(intent: &str, deployment: &str, capability: &str, floor: &str) -> JobSnapshot {
    let named_route = NamedRouteDecision {
        kind: "deployment".into(),
        ordered_ids: vec![deployment.into()],
    };
    JobSnapshot {
        id: "resp_shape_job".into(),
        app_id: "shape".into(),
        intent: intent.into(),
        consumer_core_contract: INFER_RUNTIME_CONTRACT_VERSION.into(),
        capability_contract: Some(capability.into()),
        provider: "local-provider".into(),
        deployment: deployment.into(),
        model_profile: "local-profile".into(),
        model_build: "local-build".into(),
        physical_model: "local/model".into(),
        placement: "local".into(),
        capability_level: floor.into(),
        evaluation_status: "provisional".into(),
        resource_class: "standard".into(),
        state: "succeeded".into(),
        policy: "local-first".into(),
        priority: "interactive".into(),
        constraints: json!({
            "policy": "local-first",
            "priority": "interactive",
            "provider_access_class": "standard",
            "placement": "local_only",
            "prefer": "local",
            "offline_required": true,
            "capability_floor": floor,
            "latency": "interactive",
            "max_cost_usd": 0.0,
            "fallback": "none",
            "deadline_ms": null,
            "named_route": {"kind":"deployment","ordered_ids":[deployment]}
        }),
        routing: RoutingDecision {
            capability_floor: floor.into(),
            named_route: Some(named_route),
            candidates: vec![CandidateDecision {
                deployment: deployment.into(),
                provider: "local-provider".into(),
                status: "eligible".into(),
                rank: Some(0),
                reason_codes: Vec::new(),
            }],
        },
        attempts: vec![AttemptSnapshot {
            number: 1,
            provider: "local-provider".into(),
            deployment: deployment.into(),
            outcome: "succeeded".into(),
            trigger: "initial".into(),
            error_kind: None,
            error: None,
        }],
        error: None,
    }
}

pub(crate) fn text_job() -> JobSnapshot {
    local_job(
        "text.edit",
        TEXT_EDIT_DEPLOYMENT,
        INFER_RUNTIME_RESPONSES_CAPABILITY,
        "foundational",
    )
}

pub(crate) fn speech_job() -> JobSnapshot {
    local_job(
        "speech.synthesize",
        SPEECH_DEPLOYMENT,
        INFER_RUNTIME_SPEECH_CAPABILITY,
        "capable",
    )
}

pub(crate) fn image_job() -> JobSnapshot {
    let mut job = local_job(
        "image.generate",
        "codex_gpt_5_6_luna",
        INFER_RUNTIME_RESPONSES_CAPABILITY,
        "capable",
    );
    job.provider = "codex-app-server".into();
    job.placement = "cloud".into();
    job.policy = "balanced".into();
    job.constraints = json!({
        "policy": "balanced",
        "priority": "interactive",
        "provider_access_class": "subscription",
        "placement": "cloud_only",
        "prefer": "cloud",
        "offline_required": false,
        "capability_floor": "capable",
        "latency": null,
        "max_cost_usd": 0.0,
        "fallback": "none",
        "deadline_ms": null,
        "named_route": null
    });
    job.routing.named_route = None;
    job.routing.candidates[0].provider = job.provider.clone();
    job.attempts[0].provider = job.provider.clone();
    job
}

#[test]
fn typed_sdk_job_maps_exact_core_capability_and_named_route() {
    let provenance = parse_job_snapshot(
        "resp_shape_job",
        "text.edit",
        JobPolicyProfile::LocalTextEdit,
        text_job(),
    )
    .expect("frozen SDK Job is accepted");
    assert_eq!(provenance.contract_revision, INFER_RUNTIME_CONTRACT_VERSION);
    assert_eq!(
        provenance.capability_contract.as_deref(),
        Some(INFER_RUNTIME_RESPONSES_CAPABILITY)
    );
    assert_eq!(provenance.deployment, TEXT_EDIT_DEPLOYMENT);
    let named_route = provenance
        .named_route
        .as_ref()
        .expect("text edit persists named routing");
    assert_eq!(named_route.ordered_ids.len(), 1);
    assert_eq!(named_route.ordered_ids[0], TEXT_EDIT_DEPLOYMENT);
    assert!(provenance.offline_required);
    assert_eq!(provenance.fallback, "none");
}

#[test]
fn core_capability_and_app_identity_fail_closed() {
    let mutators: [fn(&mut JobSnapshot); 3] = [
        |job: &mut JobSnapshot| job.app_id = "other".into(),
        |job: &mut JobSnapshot| job.consumer_core_contract = "unsupported-core".into(),
        |job: &mut JobSnapshot| job.capability_contract = None,
    ];
    for mutate in mutators {
        let mut job = text_job();
        mutate(&mut job);
        assert_eq!(
            parse_job_snapshot(
                "resp_shape_job",
                "text.edit",
                JobPolicyProfile::LocalTextEdit,
                job,
            ),
            Err(JobProvenanceError::InvalidResponse)
        );
    }
}

#[test]
fn local_policy_and_named_deployment_cannot_be_widened() {
    let mut fallback = speech_job();
    fallback.constraints["fallback"] = json!("allow");
    assert_eq!(
        parse_job_snapshot(
            "resp_shape_job",
            "speech.synthesize",
            JobPolicyProfile::LocalSpeech,
            fallback,
        ),
        Err(JobProvenanceError::PolicyViolation)
    );

    let mut wrong_route = speech_job();
    wrong_route
        .routing
        .named_route
        .as_mut()
        .unwrap()
        .ordered_ids = vec!["other".into()];
    assert_eq!(
        parse_job_snapshot(
            "resp_shape_job",
            "speech.synthesize",
            JobPolicyProfile::LocalSpeech,
            wrong_route,
        ),
        Err(JobProvenanceError::PolicyViolation)
    );
}

#[test]
fn source_less_image_job_remains_cloud_only_without_named_route() {
    let provenance = parse_job_snapshot(
        "resp_shape_job",
        "image.generate",
        JobPolicyProfile::CloudImageInteractive,
        image_job(),
    )
    .expect("source-less image generation keeps its separate cloud policy");
    assert!(!provenance.offline_required);
    assert!(provenance.named_route.is_none());
    assert_eq!(
        provenance.requested_provider_access_class.as_deref(),
        Some("subscription")
    );
}
