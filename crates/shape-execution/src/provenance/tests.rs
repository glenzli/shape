use super::ExternalNamedRouteProvenance;

#[test]
fn named_route_evidence_is_bounded_ordered_and_unique() {
    assert!(
        ExternalNamedRouteProvenance {
            kind: "deployment".to_owned(),
            ordered_ids: vec!["ollama_qwen3_5_4b".to_owned()],
        }
        .is_bounded()
    );
    assert!(
        !ExternalNamedRouteProvenance {
            kind: "physical_model".to_owned(),
            ordered_ids: vec!["qwen3.5:4b".to_owned()],
        }
        .is_bounded()
    );
    assert!(
        !ExternalNamedRouteProvenance {
            kind: "deployment".to_owned(),
            ordered_ids: vec!["duplicate".to_owned(), "duplicate".to_owned()],
        }
        .is_bounded()
    );
}
