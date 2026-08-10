use super::*;

fn text_id(value: &str) -> OperatorDataTypeId {
    OperatorDataTypeId::new(value).unwrap()
}

#[test]
fn working_operator_is_recoverable_without_entering_accepted_graph_history() {
    let artifact_id = ArtifactId::new();
    let revision_id = RevisionId::new();
    let mut graph = ArtifactWorkingGraph::new(artifact_id, revision_id);
    let draft = graph
        .add_operator(
            OperatorTypeId::new("text.transform").unwrap(),
            text_id("text.document"),
            text_id("text.document"),
        )
        .unwrap();
    let repeated = graph
        .add_operator(
            OperatorTypeId::new("text.transform").unwrap(),
            text_id("text.document"),
            text_id("text.document"),
        )
        .unwrap();
    assert_eq!(draft, repeated);

    let encoded = serde_json::to_string(&graph).unwrap();
    let recovered: ArtifactWorkingGraph = serde_json::from_str(&encoded).unwrap();
    recovered.validate().unwrap();
    assert_eq!(recovered, graph);
    assert_eq!(recovered.context_artifact_id(), artifact_id);
    assert_eq!(recovered.expected_revision_id(), revision_id);
}

#[test]
fn working_graph_removes_only_the_addressed_or_finished_operator() {
    let mut graph = ArtifactWorkingGraph::new(ArtifactId::new(), RevisionId::new());
    let edit = graph
        .add_operator(
            OperatorTypeId::new("text.edit").unwrap(),
            text_id("text.document"),
            text_id("text.document"),
        )
        .unwrap();
    graph
        .add_operator(
            OperatorTypeId::new("audio.speech_synthesize").unwrap(),
            text_id("text.document"),
            text_id("audio.clip"),
        )
        .unwrap();
    assert!(graph.remove_operator(edit.id()));
    assert!(!graph.remove_operator(edit.id()));
    assert!(graph.remove_operator_type("audio.speech_synthesize"));
    assert!(graph.is_empty());
}

#[test]
fn serde_loaded_identifier_is_revalidated() {
    let graph = ArtifactWorkingGraph::new(ArtifactId::new(), RevisionId::new());
    let mut value = serde_json::to_value(graph).unwrap();
    value["operators"] = serde_json::json!([{
        "id": "missing_namespace",
        "operator_type": "text.edit",
        "input_data_type": "text.document",
        "output_data_type": "text.document"
    }]);
    let malformed: ArtifactWorkingGraph = serde_json::from_value(value).unwrap();
    assert!(matches!(
        malformed.validate(),
        Err(DomainError::InvalidOperatorIdentifier { .. })
    ));
}
