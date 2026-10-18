use super::*;
use crate::{
    project_store::{DATABASE_FILE, write_manifest},
    schema,
};
use rusqlite::Connection;
use serde_json::json;

#[test]
fn prior_picture_book_schema_upgrades_without_changing_accepted_heads() {
    let root = std::env::temp_dir().join(format!("shape-card-migration-{}", uuid::Uuid::now_v7()));
    let mut store = ProjectStore::create(&root, "Prior project").unwrap();
    let artifact =
        shape_domain::Artifact::new("Existing text", ArtifactKind::TextDocument).unwrap();
    store.insert_artifact(&artifact).unwrap();
    let accepted = store
        .accept(crate::project_store::tests::successful_commit(
            &artifact, None,
        ))
        .unwrap();
    let mut expected_artifact = artifact.clone();
    expected_artifact.accepted_revision = Some(accepted.id);
    let mut metadata = store.metadata().clone();
    drop(store);
    metadata.schema_revision = schema::PICTURE_BOOK_SCHEMA_REVISION.into();
    let connection = Connection::open(root.join(DATABASE_FILE)).unwrap();
    connection
        .execute(
            "UPDATE project_singleton SET metadata_json=?1 WHERE singleton=1",
            [serde_json::to_string(&metadata).unwrap()],
        )
        .unwrap();
    drop(connection);
    write_manifest(&root, &metadata).unwrap();
    let reopened = ProjectStore::open(&root).unwrap();
    assert_eq!(
        reopened.metadata().schema_revision,
        shape_domain::SHAPE_PROJECT_SCHEMA_REVISION
    );
    assert_eq!(
        reopened.artifact(artifact.id).unwrap(),
        Some(expected_artifact)
    );
    assert_eq!(
        reopened.read_content(&accepted.content).unwrap(),
        b"accepted content"
    );
    drop(reopened);
    let again = ProjectStore::open(&root).unwrap();
    assert_eq!(again.metadata().id, metadata.id);
    drop(again);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unsupported_or_ambiguous_card_documents_do_not_cross_store_validation() {
    let mut value = json!({"schema":"shape.card-kit/1","composition":{"name":"Kit","cards":[{"id":"fox","title":"Fox","rules":"Draw"}]}});
    document(&serde_json::to_vec(&value).unwrap()).unwrap();
    value["schema"] = json!("shape.card-kit/next");
    assert!(document(&serde_json::to_vec(&value).unwrap()).is_err());
    value["schema"] = json!("shape.card-kit/1");
    value["composition"]["cards"][0]["id"] = json!("../escape");
    assert!(document(&serde_json::to_vec(&value).unwrap()).is_err());
}
