use super::*;

#[test]
fn only_portable_file_hints_are_admitted() {
    assert_eq!(safe_extension("JS"), Some("js"));
    assert_eq!(safe_extension("html"), Some("html"));
    assert_eq!(safe_extension("../js"), None);
    assert_eq!(safe_extension("exe"), None);
}

/// Rechecks an already completed Agent Job without dispatching another task.
#[test]
#[ignore = "requires an existing live Infer Agent Job and Shape credential"]
fn existing_agent_job_has_acceptable_shape_provenance() {
    let credential = std::env::var("SHAPE_TEST_INFER_CREDENTIAL").unwrap();
    let endpoint = std::env::var("SHAPE_TEST_INFER_ENDPOINT").unwrap_or_default();
    let job_id = std::env::var("SHAPE_TEST_INFER_JOB_ID").unwrap();
    let sdk = super::super::official_sdk(&endpoint, credential.into()).unwrap();
    let snapshot = sdk.job(&job_id).unwrap();
    let provenance = parse_job_snapshot(
        &job_id,
        AGENT_TASK_INTENT,
        JobPolicyProfile::AgentFileTask,
        snapshot,
    )
    .unwrap();
    assert!(
        super::super::job_provenance::approved_agent_deployment_build(
            &provenance.deployment,
            &provenance.model_build
        )
    );
    assert!(provenance.is_bounded());
}
