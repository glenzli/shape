use shape_domain::{
    AI_IMAGE_RASTER_DATA_TYPE, ArtifactId, ArtifactWorkingGraph, OperatorDataTypeId, OperatorTypeId,
};

use super::*;

fn draft(configuration: WorkingOperatorConfiguration) -> WorkingOperatorDraft {
    let mut graph = ArtifactWorkingGraph::new_source(ArtifactId::new());
    let draft = graph
        .add_source_operator(
            OperatorTypeId::new(AI_IMAGE_GENERATE_OPERATOR_TYPE).unwrap(),
            OperatorDataTypeId::new(AI_IMAGE_RASTER_DATA_TYPE).unwrap(),
        )
        .unwrap();
    assert!(graph.set_operator_configuration(draft.id(), Some(configuration)));
    graph.operators()[0].clone()
}

#[test]
fn empty_default_is_persistable_but_not_executable() {
    let draft = draft(configuration_for_ai_image_generate("", 1024, 1024, 1).unwrap());
    let state = ai_image_generate_state_from_draft(&draft).unwrap().unwrap();
    assert_eq!(state.instruction, "");
    assert_eq!((state.output.width(), state.output.height()), (1024, 1024));
    assert_eq!(state.candidate_count, 1);
    assert!(ai_image_generate_parameters_from_draft(&draft).is_err());
}

#[test]
fn authored_instruction_and_canvas_round_trip_into_exact_parameters() {
    let draft =
        draft(configuration_for_ai_image_generate("A cobalt glass bird", 1536, 1024, 3).unwrap());
    let parameters = ai_image_generate_parameters_from_draft(&draft)
        .unwrap()
        .unwrap();
    assert_eq!(parameters.instruction(), "A cobalt glass bird");
    assert_eq!(
        (parameters.output().width(), parameters.output().height()),
        (1536, 1024)
    );
    assert_eq!(parameters.candidate_count(), 3);
}

#[test]
fn unknown_fields_schemas_and_nonportable_values_fail_closed() {
    let wrong_schema = WorkingOperatorConfiguration::new(
        OperatorConfigurationSchemaId::new("shape.operator-draft.ai-image-generate@future")
            .unwrap(),
        r#"{"instruction":"bird","output":{"width":1024,"height":1024},"candidate_count":1,"constraints":[]}"#,
    )
    .unwrap();
    assert!(validate_ai_image_generate_configuration(Some(&wrong_schema)).is_err());
    assert!(configuration_for_ai_image_generate("   ", 1024, 1024, 1).is_err());
    assert!(configuration_for_ai_image_generate("bird", 0, 1024, 1).is_err());
    assert!(configuration_for_ai_image_generate("bird", 1024, 1024, 0).is_err());
    assert!(configuration_for_ai_image_generate("bird", 1024, 1024, 9).is_err());

    let unknown = WorkingOperatorConfiguration::new(
        OperatorConfigurationSchemaId::new(AI_IMAGE_GENERATE_DRAFT_SCHEMA).unwrap(),
        r#"{"instruction":"bird","output":{"width":1024,"height":1024},"candidate_count":1,"constraints":[],"provider":"hidden"}"#,
    )
    .unwrap();
    assert!(validate_ai_image_generate_configuration(Some(&unknown)).is_err());
}
