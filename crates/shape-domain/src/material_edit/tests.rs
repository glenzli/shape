use super::*;
use crate::{ArtifactId, ArtifactWorkingGraph, OperatorDataTypeId, OperatorTypeId, RevisionId};

fn draft() -> (ArtifactWorkingGraph, WorkingOperatorDraft) {
    let mut graph = ArtifactWorkingGraph::new_source(ArtifactId::new());
    let data = OperatorDataTypeId::new("image.raster").unwrap();
    let node = graph
        .add_bound_operator(
            OperatorTypeId::new("image.external_edit").unwrap(),
            data.clone(),
            data,
            WorkingInput {
                artifact_id: ArtifactId::new(),
                revision_id: RevisionId::new(),
            },
        )
        .unwrap();
    (graph, node)
}

#[test]
fn configuration_requires_exact_schema_media_and_positive_session() {
    let (mut graph, node) = draft();
    assert!(MaterialEditConfiguration::from_draft(&node).is_err());
    let mut configuration = MaterialEditConfiguration {
        editor: MaterialEditor::Shadow,
        session: 1,
    };
    graph.set_operator_configuration(node.id(), Some(configuration.encode().unwrap()));
    assert_eq!(
        MaterialEditConfiguration::from_draft(&graph.operators()[0]).unwrap(),
        configuration
    );
    configuration.editor = MaterialEditor::Echo;
    graph.set_operator_configuration(node.id(), Some(configuration.encode().unwrap()));
    assert!(MaterialEditConfiguration::from_draft(&graph.operators()[0]).is_err());
    configuration.session = 0;
    assert!(configuration.encode().is_err());
    assert!(
        serde_json::from_str::<MaterialEditConfiguration>(
            r#"{"editor":"shadow","session":1,"execute":"arbitrary"}"#
        )
        .is_err()
    );
}
