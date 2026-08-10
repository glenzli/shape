use super::*;

fn identity() -> ExecutorIdentity {
    ExecutorIdentity::new("shape.test", "1", "20260810.1").expect("valid identity")
}

#[test]
fn successful_attempt_produces_payload_free_receipt() {
    let mut job = ExecutionJob::new(
        TransformationId::new(),
        CapabilityId::new("text.test").expect("valid capability"),
    );
    let attempt = job.start(identity(), 10).expect("job starts");
    let receipt = job.succeed(attempt, 20).expect("job succeeds");

    assert_eq!(job.state(), JobState::Succeeded);
    assert_eq!(receipt.outcome, ExecutionOutcome::Succeeded);
    assert_eq!(receipt.started_at_unix_ms, 10);
    assert_eq!(receipt.completed_at_unix_ms, 20);
    assert!(receipt.executor_job_id.is_none());

    let historical_json = serde_json::to_value(&receipt).expect("receipt serializes");
    assert!(historical_json.get("executor_job_id").is_none());
    let reopened: ExecutionReceipt =
        serde_json::from_value(historical_json).expect("historical receipt reopens");
    assert!(reopened.executor_job_id.is_none());
}

#[test]
fn successful_attempt_can_link_executor_owned_provenance() {
    let mut job = ExecutionJob::new(
        TransformationId::new(),
        CapabilityId::new("text.generate").expect("valid capability"),
    );
    let attempt = job.start(identity(), 10).expect("job starts");
    let receipt = job
        .succeed_with_executor_job(attempt, 20, Some("resp_test".to_owned()))
        .expect("job succeeds");

    assert_eq!(receipt.executor_job_id.as_deref(), Some("resp_test"));
    assert_eq!(
        serde_json::to_value(&receipt).expect("receipt serializes")["executor_job_id"],
        "resp_test"
    );
}

#[test]
fn a_stale_attempt_cannot_complete_the_job() {
    let mut job = ExecutionJob::new(
        TransformationId::new(),
        CapabilityId::new("text.test").expect("valid capability"),
    );
    job.start(identity(), 10).expect("job starts");
    let error = job
        .succeed(AttemptId::new(), 20)
        .expect_err("stale attempt rejected");

    assert!(matches!(error, ExecutionError::StaleAttempt { .. }));
    assert_eq!(job.state(), JobState::Running);
}

#[test]
fn terminal_jobs_cannot_restart() {
    let mut job = ExecutionJob::new(
        TransformationId::new(),
        CapabilityId::new("text.test").expect("valid capability"),
    );
    let attempt = job.start(identity(), 10).expect("job starts");
    job.cancel(attempt, 20).expect("job cancels");

    assert!(matches!(
        job.start(identity(), 30),
        Err(ExecutionError::InvalidTransition { .. })
    ));
}
