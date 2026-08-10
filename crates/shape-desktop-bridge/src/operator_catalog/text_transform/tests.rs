use shape_domain::{OperatorDataTypeId, OperatorTypeId, WorkingOperatorDraft};

use super::*;

fn text_transform_draft() -> WorkingOperatorDraft {
    WorkingOperatorDraft::new(
        OperatorTypeId::new(TEXT_TRANSFORM_OPERATOR).unwrap(),
        OperatorDataTypeId::new("text.document").unwrap(),
        OperatorDataTypeId::new("text.document").unwrap(),
    )
}

#[test]
fn instruction_round_trips_exactly_and_empty_input_clears_configuration() {
    let exact = "  Make it warmer, but keep the title.  ";
    let configuration = configuration_for_instruction(exact).unwrap().unwrap();
    let mut graph = shape_domain::ArtifactWorkingGraph::new(
        shape_domain::ArtifactId::new(),
        shape_domain::RevisionId::new(),
    );
    let draft = graph
        .add_operator(
            OperatorTypeId::new(TEXT_TRANSFORM_OPERATOR).unwrap(),
            OperatorDataTypeId::new("text.document").unwrap(),
            OperatorDataTypeId::new("text.document").unwrap(),
        )
        .unwrap();
    graph.set_operator_configuration(draft.id(), Some(configuration));
    assert_eq!(
        instruction_from_draft(&graph.operators()[0]).unwrap(),
        exact
    );
    assert!(configuration_for_instruction("  ").unwrap().is_none());
}

#[test]
fn unknown_fields_wrong_schema_and_oversized_instructions_fail_closed() {
    let wrong_schema = WorkingOperatorConfiguration::new(
        OperatorConfigurationSchemaId::new("shape.operator-draft.other@1").unwrap(),
        r#"{"instruction":"clear"}"#,
    )
    .unwrap();
    assert!(decode(&wrong_schema).is_err());
    let unknown_field = WorkingOperatorConfiguration::new(
        OperatorConfigurationSchemaId::new(TEXT_TRANSFORM_DRAFT_SCHEMA).unwrap(),
        r#"{"instruction":"clear","provider":"forbidden"}"#,
    )
    .unwrap();
    assert!(decode(&unknown_field).is_err());
    assert!(configuration_for_instruction(&"x".repeat(MAX_INSTRUCTION_BYTES + 1)).is_err());
    assert!(
        instruction_from_draft(&text_transform_draft())
            .unwrap()
            .is_empty()
    );
}
