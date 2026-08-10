use super::*;

#[test]
fn capability_ids_require_a_namespace() {
    assert!(CapabilityId::new("text.literal").is_ok());
    assert!(matches!(
        CapabilityId::new("literal"),
        Err(ExecutionError::InvalidCapabilityId)
    ));
}

#[test]
fn identity_fields_are_portable_ascii() {
    assert!(ExecutorIdentity::new("shape.builtin", "0.1.0", "20260810.1").is_ok());
    assert!(matches!(
        ExecutorIdentity::new("形状", "0.1.0", "20260810.1"),
        Err(ExecutionError::InvalidExecutorIdentity)
    ));
}

#[test]
fn materialized_inputs_verify_identity_and_cannot_fall_back_to_references() {
    let bytes = b"verified input".to_vec();
    let content = ContentRef::new(
        ContentDigest::from_bytes(&bytes),
        "application/octet-stream",
        bytes.len() as u64,
    )
    .unwrap();
    assert!(ExecutionInput::materialized(content.clone(), bytes.clone()).is_ok());
    assert!(ExecutionInput::materialized(content.clone(), b"changed".to_vec()).is_err());

    let request = ExecutionRequest::new_materialized(
        TransformationId::new(),
        CapabilityId::new("image.raster.crop").unwrap(),
        vec![ExecutionInput::reference(content)],
        Vec::new(),
        "image/png",
    );
    assert!(matches!(request, Err(ExecutionError::InvalidInput)));
}
