use shape_domain::{OperatorDataTypeId, OperatorTypeId, WorkingOperatorDraft};

use super::*;
use crate::operator_catalog::{TEXT_EDIT_OPERATOR, TEXT_TRANSFORM_OPERATOR};

fn writing_draft() -> WorkingOperatorDraft {
    WorkingOperatorDraft::new(
        OperatorTypeId::new(TEXT_EDIT_OPERATOR).unwrap(),
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
            OperatorTypeId::new(TEXT_EDIT_OPERATOR).unwrap(),
            OperatorDataTypeId::new("text.document").unwrap(),
            OperatorDataTypeId::new("text.document").unwrap(),
        )
        .unwrap();
    graph.set_operator_configuration(draft.id(), Some(configuration));
    assert_eq!(
        instruction_from_draft(&graph.operators()[0]).unwrap(),
        exact
    );
    assert_eq!(mode_from_draft(&graph.operators()[0]).unwrap(), "rewrite");
    assert!(configuration_for_instruction("  ").unwrap().is_none());
}

#[test]
fn all_modes_round_trip_in_one_text_transform_schema() {
    for mode in ["rewrite", "expand", "polish", "shorten", "summarize"] {
        let configuration = configuration_for_mode_and_instruction(mode, "  Keep this exact.  ")
            .unwrap()
            .unwrap();
        assert_eq!(configuration.schema().as_str(), TEXT_TRANSFORM_DRAFT_SCHEMA);
        let decoded = decode(&configuration).unwrap();
        assert_eq!(decoded.mode.as_str(), mode);
        assert_eq!(decoded.instruction, "  Keep this exact.  ");
    }
}

#[test]
fn studio_intent_round_trips_tone_style_and_variant_count() {
    let configuration = configuration_for_studio(
        "summarize",
        "Keep every quoted number.",
        "confident",
        "professional",
        3,
    )
    .unwrap()
    .unwrap();
    let mut graph = shape_domain::ArtifactWorkingGraph::new(
        shape_domain::ArtifactId::new(),
        shape_domain::RevisionId::new(),
    );
    let draft = graph
        .add_operator(
            OperatorTypeId::new(TEXT_EDIT_OPERATOR).unwrap(),
            OperatorDataTypeId::new("text.document").unwrap(),
            OperatorDataTypeId::new("text.document").unwrap(),
        )
        .unwrap();
    graph.set_operator_configuration(draft.id(), Some(configuration));
    let draft = &graph.operators()[0];
    assert_eq!(mode_from_draft(draft).unwrap(), "summarize");
    assert_eq!(tone_from_draft(draft).unwrap(), "confident");
    assert_eq!(style_from_draft(draft).unwrap(), "professional");
    assert_eq!(variant_count_from_draft(draft).unwrap(), 3);
    assert!(
        compiled_instruction_from_draft(draft)
            .unwrap()
            .contains("Tone: confident. Style: professional.")
    );
}

#[test]
fn legacy_configuration_reopens_as_rewrite_without_changing_authored_text() {
    let legacy = WorkingOperatorConfiguration::new(
        OperatorConfigurationSchemaId::new(LEGACY_TEXT_TRANSFORM_DRAFT_SCHEMA).unwrap(),
        r#"{"instruction":"  Preserve legacy whitespace.  "}"#,
    )
    .unwrap();
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
    graph.set_operator_configuration(draft.id(), Some(legacy));

    assert_eq!(
        instruction_from_draft(&graph.operators()[0]).unwrap(),
        "  Preserve legacy whitespace.  "
    );
    assert_eq!(mode_from_draft(&graph.operators()[0]).unwrap(), "rewrite");
}

#[test]
fn unknown_modes_fields_schemas_and_oversized_instructions_fail_closed() {
    let wrong_schema = WorkingOperatorConfiguration::new(
        OperatorConfigurationSchemaId::new("shape.operator-draft.other@1").unwrap(),
        r#"{"mode":"rewrite","instruction":"clear"}"#,
    )
    .unwrap();
    assert!(decode(&wrong_schema).is_err());
    let unknown_field = WorkingOperatorConfiguration::new(
        OperatorConfigurationSchemaId::new(TEXT_TRANSFORM_DRAFT_SCHEMA).unwrap(),
        r#"{"mode":"rewrite","instruction":"clear","provider":"forbidden"}"#,
    )
    .unwrap();
    assert!(decode(&unknown_field).is_err());
    let unknown_mode = WorkingOperatorConfiguration::new(
        OperatorConfigurationSchemaId::new(TEXT_TRANSFORM_DRAFT_SCHEMA).unwrap(),
        r#"{"mode":"translate","instruction":"clear","tone":"neutral","style":"natural","variant_count":1}"#,
    )
    .unwrap();
    assert!(decode(&unknown_mode).is_err());
    assert!(configuration_for_mode_and_instruction("translate", "clear").is_err());
    assert!(configuration_for_studio("rewrite", "", "angry", "natural", 1).is_err());
    assert!(configuration_for_studio("rewrite", "", "neutral", "academic", 1).is_err());
    assert!(configuration_for_studio("rewrite", "", "neutral", "natural", 0).is_err());
    assert!(configuration_for_studio("rewrite", "", "neutral", "natural", 5).is_err());
    assert!(configuration_for_instruction(&"x".repeat(MAX_INSTRUCTION_BYTES + 1)).is_err());
    assert!(instruction_from_draft(&writing_draft()).unwrap().is_empty());
    assert_eq!(mode_from_draft(&writing_draft()).unwrap(), "rewrite");
}
