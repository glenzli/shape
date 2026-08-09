//! `SQLite` schema owned by the project store.

use rusqlite::Connection;

use crate::StoreError;

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
