use shape_domain::{
    ArtifactId, ArtifactWorkingGraph, OperatorConfigurationSchemaId, OperatorDataTypeId,
    OperatorTypeId, RevisionId, WorkingOperatorConfiguration, WorkingOperatorDraft,
};

use super::*;

fn image_resize_draft() -> WorkingOperatorDraft {
    WorkingOperatorDraft::new(
        OperatorTypeId::new(IMAGE_RESIZE_OPERATOR).unwrap(),
        OperatorDataTypeId::new("image.raster").unwrap(),
        OperatorDataTypeId::new("image.raster").unwrap(),
    )
}

fn configured_draft(configuration: WorkingOperatorConfiguration) -> WorkingOperatorDraft {
    let mut graph = ArtifactWorkingGraph::new(ArtifactId::new(), RevisionId::new());
    let draft = graph
        .add_operator(
            OperatorTypeId::new(IMAGE_RESIZE_OPERATOR).unwrap(),
            OperatorDataTypeId::new("image.raster").unwrap(),
            OperatorDataTypeId::new("image.raster").unwrap(),
        )
        .unwrap();
    assert!(graph.set_operator_configuration(draft.id(), Some(configuration)));
    graph.operators()[0].clone()
}

#[test]
fn exact_resize_configuration_round_trips_to_domain_parameters() {
    for (aspect, resampling) in [
        ("stretch", "nearest"),
        ("fit_within", "triangle"),
        ("fit_within", "catmull_rom"),
        ("fit_within", "lanczos3"),
    ] {
        let configuration = configuration_for_image_resize(1_920, 1_080, aspect, resampling)
            .expect("valid resize configures");
        assert_eq!(configuration.schema().as_str(), IMAGE_RESIZE_DRAFT_SCHEMA);
        let resize = image_resize_from_draft(&configured_draft(configuration))
            .unwrap()
            .expect("resize draft decodes");
        assert_eq!(resize.target().width(), 1_920);
        assert_eq!(resize.target().height(), 1_080);
        assert_eq!(aspect_policy_key(resize.aspect_policy()), aspect);
        assert_eq!(resampling_key(resize.resampling()), resampling);
    }
}

#[test]
fn invalid_dimensions_modes_fields_and_schema_fail_closed() {
    assert!(configuration_for_image_resize(0, 100, "fit_within", "lanczos3").is_err());
    assert!(configuration_for_image_resize(100, 100, "crop", "lanczos3").is_err());
    assert!(configuration_for_image_resize(100, 100, "fit_within", "automatic").is_err());

    let unknown_field = WorkingOperatorConfiguration::new(
        OperatorConfigurationSchemaId::new(IMAGE_RESIZE_DRAFT_SCHEMA).unwrap(),
        r#"{"target":{"width":320,"height":240},"aspect_policy":"fit_within","resampling":"lanczos3","provider":"shadow"}"#,
    )
    .unwrap();
    assert!(validate_image_resize_configuration(Some(&unknown_field)).is_err());

    let wrong_schema = WorkingOperatorConfiguration::new(
        OperatorConfigurationSchemaId::new("shape.operator-draft.image-resize@other").unwrap(),
        r#"{"target":{"width":320,"height":240},"aspect_policy":"fit_within","resampling":"lanczos3"}"#,
    )
    .unwrap();
    assert!(validate_image_resize_configuration(Some(&wrong_schema)).is_err());
    assert!(validate_image_resize_configuration(None).is_err());
    assert!(
        image_resize_from_draft(&image_resize_draft())
            .unwrap()
            .is_none()
    );
}
