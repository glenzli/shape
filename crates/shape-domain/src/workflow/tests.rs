use super::*;

#[test]
fn public_catalog_advertises_only_general_material_workflows() {
    let templates = workflow_templates();
    assert!(
        templates
            .iter()
            .any(|template| template.id == "material_edit")
    );
    assert!(
        templates
            .iter()
            .all(|template| !matches!(template.output_kind, "card_kit" | "picture_book"))
    );
}
