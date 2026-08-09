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
