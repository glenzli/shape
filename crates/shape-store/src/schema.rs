//! `SQLite` schema owned by the project store.

use rusqlite::Connection;
use shape_domain::{ProjectMetadata, SHAPE_PROJECT_SCHEMA_REVISION};

use crate::StoreError;

pub(super) const INITIAL_SCHEMA_REVISION: &str = "20260810.1";
pub(super) const SCENE_SCHEMA_REVISION: &str = "20260811.1";

const SCENE_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS scenes (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        accepted_revision TEXT NULL
    );

    CREATE TABLE IF NOT EXISTS scene_revisions (
        id TEXT PRIMARY KEY,
        scene_id TEXT NOT NULL REFERENCES scenes(id),
        parent_revision TEXT NULL,
        revision_json TEXT NOT NULL
    );
";

pub(super) fn initialize(connection: &Connection) -> Result<(), StoreError> {
    connection.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = FULL;

        CREATE TABLE project_singleton (
            singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
            metadata_json TEXT NOT NULL
        );

        CREATE TABLE artifacts (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            kind_json TEXT NOT NULL,
            accepted_revision TEXT NULL
        );

        CREATE TABLE transformations (
            id TEXT PRIMARY KEY,
            artifact_id TEXT NOT NULL REFERENCES artifacts(id),
            transformation_json TEXT NOT NULL
        );

        CREATE TABLE execution_receipts (
            attempt_id TEXT PRIMARY KEY,
            transformation_id TEXT NOT NULL REFERENCES transformations(id),
            receipt_json TEXT NOT NULL
        );

        CREATE TABLE artifact_revisions (
            id TEXT PRIMARY KEY,
            artifact_id TEXT NOT NULL REFERENCES artifacts(id),
            transformation_id TEXT NOT NULL REFERENCES transformations(id),
            revision_json TEXT NOT NULL
        );
        ",
    )?;
    connection.execute_batch(SCENE_SCHEMA)?;
    Ok(())
}

pub(super) fn prepare_connection(connection: &Connection) -> Result<(), StoreError> {
    connection.execute_batch(
        "
        PRAGMA foreign_keys = ON;
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = FULL;
        ",
    )?;
    Ok(())
}

/// Returns whether a manifest revision can be upgraded without reinterpretation.
pub(super) fn supports_migration(revision: &str) -> bool {
    matches!(
        revision,
        INITIAL_SCHEMA_REVISION | SCENE_SCHEMA_REVISION | SHAPE_PROJECT_SCHEMA_REVISION
    )
}

/// Additively upgrades initial and Scene-era projects to the current schema.
///
/// The `SQLite` transaction commits before the manifest is atomically replaced.
/// If the process stops between those boundaries, the current metadata and
/// idempotent table creation let the next open safely finish the manifest step.
pub(super) fn migrate_to_current_schema(
    connection: &mut Connection,
) -> Result<ProjectMetadata, StoreError> {
    let metadata_json: String = connection.query_row(
        "SELECT metadata_json FROM project_singleton WHERE singleton = 1",
        [],
        |row| row.get(0),
    )?;
    let mut metadata: ProjectMetadata = serde_json::from_str(&metadata_json)?;
    if !supports_migration(&metadata.schema_revision) {
        return Err(StoreError::SchemaMismatch {
            actual: metadata.schema_revision,
            required: SHAPE_PROJECT_SCHEMA_REVISION,
        });
    }

    let transaction = connection.transaction()?;
    transaction.execute_batch(SCENE_SCHEMA)?;
    if metadata.schema_revision != SHAPE_PROJECT_SCHEMA_REVISION {
        SHAPE_PROJECT_SCHEMA_REVISION.clone_into(&mut metadata.schema_revision);
        transaction.execute(
            "UPDATE project_singleton SET metadata_json = ?1 WHERE singleton = 1",
            [serde_json::to_string(&metadata)?],
        )?;
    }
    transaction.commit()?;
    Ok(metadata)
}
