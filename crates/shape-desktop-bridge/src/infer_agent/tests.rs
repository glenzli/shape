use super::output_identity;

#[test]
fn output_keeps_supported_code_extension_without_exposing_source_path() {
    assert_eq!(
        output_identity("scene.js"),
        ("scene · Agent.js".into(), "js")
    );
    assert_eq!(
        output_identity("notes"),
        ("notes · Agent.txt".into(), "txt")
    );
    assert_eq!(
        output_identity("odd.exe"),
        ("odd · Agent.txt".into(), "txt")
    );
}
