use super::*;
use serde_json::json;

#[test]
fn partial_overrides_keep_explicit_root_values_and_leave_mode_defaults_unset() {
    let root: CardLayoutConfig = serde_json::from_value(json!({"mode":"auto","padding_percent":7,"background_color":"#A1b2C3","text_scale_percent":150})).unwrap();
    let card: CardLayoutConfig =
        serde_json::from_value(json!({"mode":"image","border":false})).unwrap();
    let merged = root.with_overrides(Some(&card));
    merged.validate().unwrap();
    assert_eq!(merged.mode, Some(CardLayoutMode::Image));
    assert_eq!(merged.padding_percent, Some(7));
    assert_eq!(merged.text_scale_percent, Some(150));
    assert_eq!(merged.text_align, None);
    assert_eq!(
        serde_json::to_value(CardLayoutConfig::default()).unwrap(),
        json!({})
    );
}

#[test]
fn colors_scales_padding_and_presets_are_bounded() {
    for value in [
        json!({"padding_percent":21}),
        json!({"text_scale_percent":49}),
        json!({"text_scale_percent":301}),
        json!({"text_color":"red"}),
        json!({"background_color":"#ff00zz"}),
    ] {
        let config: CardLayoutConfig = serde_json::from_value(value).unwrap();
        assert!(config.validate().is_err());
    }
    assert!(serde_json::from_value::<CardLayoutConfig>(json!({"mode":"canvas"})).is_err());
    assert!(serde_json::from_value::<CardLayoutConfig>(json!({"font":"web"})).is_err());
}
