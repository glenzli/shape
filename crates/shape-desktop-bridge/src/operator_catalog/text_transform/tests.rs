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
    assert!(compiled_instruction_from_draft(draft).unwrap().contains(
        "Tone: confident at balanced intensity. Audience: general audience. Style: professional."
    ));
}

#[test]
fn visual_tone_mix_custom_snapshot_and_audience_round_trip() {
    let expression = r#"{"tones":[{"kind":"preset","preset":"warm"},{"kind":"custom","name":"Quiet conviction","instruction":"Sound certain without sounding forceful.","example":"This is the right direction; we can proceed carefully.","visual":"ascent"}],"intensity":"subtle","audience":{"kind":"preset","preset":"colleague"}}"#;
    let configuration = configuration_for_expression_studio(
        "polish",
        "Keep the exact dates.",
        expression,
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

    let projected: serde_json::Value =
        serde_json::from_str(&expression_json_from_draft(draft).unwrap()).unwrap();
    assert_eq!(
        projected,
        serde_json::from_str::<serde_json::Value>(expression).unwrap()
    );
    let compiled = compiled_instruction_from_draft(draft).unwrap();
    assert!(compiled.contains(
        "Tone: primarily warm, with custom tone ‘Quiet conviction’ (direction: Sound certain without sounding forceful. Example: “This is the right direction; we can proceed carefully.”) as a supporting tone at subtle intensity."
    ));
    assert!(compiled.contains("Audience: a colleague. Style: professional."));
}

#[test]
fn expression_rejects_ambiguous_or_unbounded_authored_state() {
    let invalid_expressions = [
        r#"{"tones":[],"intensity":"balanced","audience":{"kind":"preset","preset":"general"}}"#,
        r#"{"tones":[{"kind":"preset","preset":"warm"},{"kind":"preset","preset":"warm"}],"intensity":"balanced","audience":{"kind":"preset","preset":"general"}}"#,
        r#"{"tones":[{"kind":"preset","preset":"neutral"},{"kind":"preset","preset":"warm"}],"intensity":"balanced","audience":{"kind":"preset","preset":"general"}}"#,
        r#"{"tones":[{"kind":"custom","name":"","instruction":"clear","visual":"ripple"}],"intensity":"balanced","audience":{"kind":"preset","preset":"general"}}"#,
        r#"{"tones":[{"kind":"preset","preset":"warm"}],"intensity":"balanced","audience":{"kind":"custom","name":"Team","instruction":""}}"#,
        r#"{"tones":[{"kind":"preset","preset":"warm","provider":"forbidden"}],"intensity":"balanced","audience":{"kind":"preset","preset":"general"}}"#,
    ];
    for expression in invalid_expressions {
        assert!(
            configuration_for_expression_studio("rewrite", "Keep facts.", expression, "natural", 1)
                .is_err()
        );
    }
    let oversized = format!(
        r#"{{"tones":[{{"kind":"custom","name":"Mine","instruction":"{}","visual":"glow"}}],"intensity":"strong","audience":{{"kind":"preset","preset":"general"}}}}"#,
        "x".repeat(MAX_CUSTOM_INSTRUCTION_BYTES + 1)
    );
    assert!(
        configuration_for_expression_studio("rewrite", "Keep facts.", &oversized, "natural", 1)
            .is_err()
    );
    let oversized_example = format!(
        r#"{{"tones":[{{"kind":"custom","name":"Mine","instruction":"Stay measured.","example":"{}","visual":"glow"}}],"intensity":"strong","audience":{{"kind":"preset","preset":"general"}}}}"#,
        "x".repeat(MAX_CUSTOM_EXAMPLE_BYTES + 1)
    );
    assert!(
        configuration_for_expression_studio(
            "rewrite",
            "Keep facts.",
            &oversized_example,
            "natural",
            1,
        )
        .is_err()
    );
}

#[test]
fn previous_expression_schema_reopens_custom_tone_without_an_example() {
    let previous = WorkingOperatorConfiguration::new(
        OperatorConfigurationSchemaId::new(PREVIOUS_EXPRESSION_TEXT_TRANSFORM_DRAFT_SCHEMA)
            .unwrap(),
        r#"{"mode":"polish","instruction":"Keep names.","expression":{"tones":[{"kind":"custom","name":"Quiet conviction","instruction":"Stay certain without force.","visual":"ascent"}],"intensity":"balanced","audience":{"kind":"preset","preset":"general"}},"style":"natural","variant_count":1}"#,
    )
    .unwrap();
    assert!(decode(&previous).is_ok());
}

#[test]
fn previous_studio_schema_maps_to_balanced_general_expression() {
    let previous = WorkingOperatorConfiguration::new(
        OperatorConfigurationSchemaId::new(PREVIOUS_STUDIO_TEXT_TRANSFORM_DRAFT_SCHEMA).unwrap(),
        r#"{"mode":"polish","instruction":"Keep names.","tone":"warm","style":"concise","variant_count":3}"#,
    )
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
    graph.set_operator_configuration(draft.id(), Some(previous));
    let draft = &graph.operators()[0];

    assert_eq!(tone_from_draft(draft).unwrap(), "warm");
    assert!(
        compiled_instruction_from_draft(draft)
            .unwrap()
            .contains("warm at balanced intensity. Audience: general audience.")
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
