use super::*;

#[test]
fn execution_ids_round_trip_through_text() {
    let job = JobId::new();
    let parsed = job.to_string().parse::<JobId>().expect("valid job id");
    assert_eq!(job, parsed);
}
