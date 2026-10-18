//! Small product template catalog. Concrete consumers own drafts, execution and acceptance.
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowTemplate {
    pub id: &'static str,
    pub output_kind: &'static str,
    pub steps: &'static [&'static str],
    pub slots: &'static [&'static str],
}

pub fn workflow_templates() -> Vec<WorkflowTemplate> {
    vec![WorkflowTemplate {
        id: "material_edit",
        output_kind: "image_raster",
        steps: &["source", "edit", "review", "accept"],
        slots: &["accepted_material"],
    }]
}

#[cfg(test)]
mod tests;
