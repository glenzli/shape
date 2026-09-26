use super::*;
#[test]
fn drafts_allow_empty_prompt_but_never_reference_or_provider_fields() {
    assert_eq!(
        decode(Some(&configuration("", "sound_effect", 5, 42).unwrap()))
            .unwrap()
            .prompt,
        ""
    );
    assert!(configuration("Rain", "open_small", 5, 42).is_err());
    assert!(configuration("Rain", "short_music", 31, 42).is_err());
    let config = configuration("Piano", "short_music", 9, 77).unwrap();
    let p = decode(Some(&config)).unwrap();
    assert_eq!(p.seed, 77);
    assert_eq!(p.duration_seconds, 9);
    let unknown = WorkingOperatorConfiguration::new(
        OperatorConfigurationSchemaId::new(SCHEMA).unwrap(),
        config.json().replace('{', "{\"references\":[],"),
    )
    .unwrap();
    assert!(decode(Some(&unknown)).is_err());
}
