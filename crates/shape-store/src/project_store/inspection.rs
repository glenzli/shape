//! Current-schema inspection: one bounded read transaction, no upgrades or object writes.
use super::{
    BundleManifest, DATABASE_FILE, MANIFEST_FILE, MANIFEST_SCHEMA, ProjectSnapshot, ProjectStore,
};
use crate::{StoreError, objects::ObjectStore};
use rusqlite::{Connection, OpenFlags};
use shape_domain::{ArtifactWorkingGraph, ProjectMetadata, SHAPE_PROJECT_SCHEMA_REVISION};
use std::{fs::File, io::Read, path::Path, time::Duration};

const MAX_MANIFEST_BYTES: u64 = 65_536;
const MAX_COLLECTION_ROWS: u64 = 1024;
const MAX_METADATA_BYTES: u64 = 4 * 1024 * 1024;

/// Persisted heads and saved graphs from the same `SQLite` read transaction.
/// Stale graphs remain visible for diagnostics; transient candidates are not persisted here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectInspection {
    pub snapshot: ProjectSnapshot,
    pub working_graphs: Vec<ArtifactWorkingGraph>,
}

impl ProjectStore {
    /// Inspects a current-schema bundle without migration, payload reads or logical writes.
    /// `SQLite` may use its ordinary WAL locking sidecars while another process is editing.
    ///
    /// # Errors
    /// Rejects old/mismatched schemas, malformed data, busy databases, more than 1024 rows in
    /// any inspected collection, or more than 4 MiB of combined inspected metadata.
    pub fn inspect(root: impl AsRef<Path>) -> Result<ProjectInspection, StoreError> {
        let store = open_for_inspection(root.as_ref())?;
        let transaction = store.connection.unchecked_transaction()?;
        let mut budget = MAX_METADATA_BYTES;
        check_budget(
            &store.connection,
            "SELECT COUNT(*), COALESCE(SUM(bytes),0) FROM (SELECT LENGTH(CAST(metadata_json AS BLOB)) AS bytes FROM project_singleton LIMIT ?1)",
            &mut budget,
        )?;
        let metadata_json: String = store.connection.query_row(
            "SELECT metadata_json FROM project_singleton WHERE singleton = 1",
            [],
            |row| row.get(0),
        )?;
        let metadata: ProjectMetadata = serde_json::from_str(&metadata_json)?;
        if metadata != store.metadata {
            return Err(StoreError::MetadataMismatch);
        }
        check_budget(
            &store.connection,
            "SELECT COUNT(*), COALESCE(SUM(bytes),0) FROM (SELECT LENGTH(CAST(id AS BLOB)) + LENGTH(CAST(name AS BLOB)) + LENGTH(CAST(kind_json AS BLOB)) + COALESCE(LENGTH(accepted_revision),0) AS bytes FROM artifacts LIMIT ?1)",
            &mut budget,
        )?;
        check_budget(
            &store.connection,
            "SELECT COUNT(*), COALESCE(SUM(bytes),0) FROM (SELECT LENGTH(CAST(id AS BLOB)) + LENGTH(CAST(name AS BLOB)) + COALESCE(LENGTH(accepted_revision),0) AS bytes FROM scenes LIMIT ?1)",
            &mut budget,
        )?;
        check_budget(
            &store.connection,
            "SELECT COUNT(*), COALESCE(SUM(bytes),0) FROM (SELECT LENGTH(CAST(artifact_id AS BLOB)) + LENGTH(CAST(graph_json AS BLOB)) AS bytes FROM artifact_working_graphs LIMIT ?1)",
            &mut budget,
        )?;
        let snapshot = store.snapshot()?;
        let mut statement = store
            .connection
            .prepare("SELECT artifact_id FROM artifact_working_graphs ORDER BY artifact_id")?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let mut working_graphs = Vec::with_capacity(ids.len());
        for value in ids {
            let id = value
                .parse()
                .map_err(|_| StoreError::InvalidInspection("invalid graph artifact identity"))?;
            if !snapshot.artifacts.iter().any(|artifact| artifact.id == id) {
                return Err(StoreError::UnknownArtifact(id));
            }
            working_graphs.push(
                store
                    .artifact_working_graph(id)?
                    .ok_or(StoreError::InvalidInspection("saved graph disappeared"))?,
            );
        }
        drop(statement);
        transaction.rollback()?;
        Ok(ProjectInspection {
            snapshot,
            working_graphs,
        })
    }
}

fn open_for_inspection(root: &Path) -> Result<ProjectStore, StoreError> {
    if !root.is_dir() {
        return Err(StoreError::BundleMissing(root.to_owned()));
    }
    let manifest_path = root.join(MANIFEST_FILE);
    if !std::fs::metadata(&manifest_path)?.is_file() {
        return Err(StoreError::InvalidInspection(
            "manifest must be a regular file",
        ));
    }
    let mut bytes = Vec::new();
    File::open(&manifest_path)?
        .take(MAX_MANIFEST_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(StoreError::InspectionLimit {
            resource: "manifest bytes",
            maximum: MAX_MANIFEST_BYTES,
        });
    }
    let manifest: BundleManifest = serde_json::from_slice(&bytes)?;
    if manifest.schema != MANIFEST_SCHEMA
        || manifest.metadata.schema_revision != SHAPE_PROJECT_SCHEMA_REVISION
    {
        return Err(StoreError::SchemaMismatch {
            actual: manifest.metadata.schema_revision,
            required: SHAPE_PROJECT_SCHEMA_REVISION,
        });
    }
    let connection = Connection::open_with_flags(
        root.join(DATABASE_FILE),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    connection.busy_timeout(Duration::from_millis(250))?;
    connection.execute_batch("PRAGMA query_only = ON;")?;
    Ok(ProjectStore {
        root: root.to_owned(),
        metadata: manifest.metadata,
        connection,
        objects: ObjectStore::open(root)?,
    })
}

fn check_budget(connection: &Connection, query: &str, budget: &mut u64) -> Result<(), StoreError> {
    let (rows, bytes): (i64, i64) = connection.query_row(
        query,
        [i64::try_from(MAX_COLLECTION_ROWS + 1).map_err(|_| StoreError::IntegerOutOfRange)?],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let rows = u64::try_from(rows).map_err(|_| StoreError::IntegerOutOfRange)?;
    let bytes = u64::try_from(bytes).map_err(|_| StoreError::IntegerOutOfRange)?;
    if rows > MAX_COLLECTION_ROWS {
        return Err(StoreError::InspectionLimit {
            resource: "collection rows",
            maximum: MAX_COLLECTION_ROWS,
        });
    }
    *budget = budget
        .checked_sub(bytes)
        .ok_or(StoreError::InspectionLimit {
            resource: "metadata bytes",
            maximum: MAX_METADATA_BYTES,
        })?;
    Ok(())
}

#[cfg(test)]
mod tests;
