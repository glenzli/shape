//! Project bundle lifecycle and atomic accepted-head transactions.

use std::{
    fmt,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use shape_domain::{
    Artifact, ArtifactId, ArtifactKind, ArtifactRevision, ContentRef, ProjectMetadata, RevisionId,
    SHAPE_PROJECT_SCHEMA_REVISION, Transformation, TransformationId,
};
use shape_execution::{ExecutionOutcome, ExecutionReceipt};
use uuid::Uuid;

use crate::{StoreError, objects::ObjectStore, schema};

const MANIFEST_SCHEMA: &str = "shape.project-bundle";
const MANIFEST_FILE: &str = "manifest.json";
const DATABASE_FILE: &str = "project.sqlite";

/// Candidate data and provenance to publish as one accepted artifact revision.
pub struct AcceptedCommit {
    pub artifact_id: ArtifactId,
    pub expected_head: Option<RevisionId>,
    pub transformation: Transformation,
    pub receipt: ExecutionReceipt,
    pub output_bytes: Vec<u8>,
    pub output_media_type: String,
}

/// Provenance and output to publish as the first accepted revision of a new artifact.
pub struct NewArtifactCommit {
    pub artifact: Artifact,
    pub transformation: Transformation,
    pub receipt: ExecutionReceipt,
    pub output_bytes: Vec<u8>,
    pub output_media_type: String,
}

impl fmt::Debug for NewArtifactCommit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NewArtifactCommit")
            .field("artifact", &self.artifact)
            .field("transformation_id", &self.transformation.id)
            .field("receipt", &self.receipt)
            .field("output_byte_length", &self.output_bytes.len())
            .field("output_media_type", &self.output_media_type)
            .finish()
    }
}

impl fmt::Debug for AcceptedCommit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AcceptedCommit")
            .field("artifact_id", &self.artifact_id)
            .field("expected_head", &self.expected_head)
            .field("transformation_id", &self.transformation.id)
            .field("receipt", &self.receipt)
            .field("output_byte_length", &self.output_bytes.len())
            .field("output_media_type", &self.output_media_type)
            .finish()
    }
}

/// Read-only project metadata and current artifact heads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSnapshot {
    pub metadata: ProjectMetadata,
    pub artifacts: Vec<Artifact>,
}

/// Exclusive persistence facade for one open `.shape` bundle.
#[derive(Debug)]
pub struct ProjectStore {
    root: PathBuf,
    metadata: ProjectMetadata,
    connection: Connection,
    objects: ObjectStore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct BundleManifest {
    schema: String,
    metadata: ProjectMetadata,
}

impl ProjectStore {
    /// Creates a new project bundle at a path that does not yet exist.
    ///
    /// # Errors
    ///
    /// Returns an error for an existing path, invalid metadata, or durable write failure.
    pub fn create(root: impl AsRef<Path>, name: impl Into<String>) -> Result<Self, StoreError> {
        let root = root.as_ref().to_owned();
        if root.exists() {
            return Err(StoreError::BundleExists(root));
        }
        let metadata = ProjectMetadata::new(name)?;
        fs::create_dir_all(&root)?;
        fs::create_dir_all(root.join("previews"))?;
        let objects = ObjectStore::create(&root)?;
        let connection = Connection::open(root.join(DATABASE_FILE))?;
        schema::initialize(&connection)?;

        let metadata_json = serde_json::to_string(&metadata)?;
        connection.execute(
            "INSERT INTO project_singleton (singleton, metadata_json) VALUES (1, ?1)",
            [metadata_json],
        )?;
        write_manifest(&root, &metadata)?;
        Ok(Self {
            root,
            metadata,
            connection,
            objects,
        })
    }

    /// Opens and validates an existing project bundle.
    ///
    /// # Errors
    ///
    /// Returns an error when the manifest, schema, database, or object root is invalid.
    pub fn open(root: impl AsRef<Path>) -> Result<Self, StoreError> {
        let root = root.as_ref().to_owned();
        if !root.is_dir() {
            return Err(StoreError::BundleMissing(root));
        }
        let manifest: BundleManifest =
            serde_json::from_slice(&fs::read(root.join(MANIFEST_FILE))?)?;
        if manifest.schema != MANIFEST_SCHEMA
            || manifest.metadata.schema_revision != SHAPE_PROJECT_SCHEMA_REVISION
        {
            return Err(StoreError::SchemaMismatch {
                actual: manifest.metadata.schema_revision,
                required: SHAPE_PROJECT_SCHEMA_REVISION,
            });
        }
        let connection = Connection::open(root.join(DATABASE_FILE))?;
        schema::prepare_connection(&connection)?;
        let metadata_json: String = connection.query_row(
            "SELECT metadata_json FROM project_singleton WHERE singleton = 1",
            [],
            |row| row.get(0),
        )?;
        let metadata: ProjectMetadata = serde_json::from_str(&metadata_json)?;
        if metadata != manifest.metadata {
            return Err(StoreError::MetadataMismatch);
        }
        let objects = ObjectStore::open(&root)?;
        Ok(Self {
            root,
            metadata,
            connection,
            objects,
        })
    }

    /// Returns the bundle root path.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns immutable project metadata.
    #[must_use]
    pub const fn metadata(&self) -> &ProjectMetadata {
        &self.metadata
    }

    /// Inserts a new stable artifact without accepted content.
    ///
    /// # Errors
    ///
    /// Returns an error when its identity already exists or persistence fails.
    pub fn insert_artifact(&self, artifact: &Artifact) -> Result<(), StoreError> {
        let kind_json = serde_json::to_string(&artifact.kind)?;
        let result = self.connection.execute(
            "INSERT INTO artifacts (id, name, kind_json, accepted_revision) VALUES (?1, ?2, ?3, ?4)",
            params![
                artifact.id.to_string(),
                artifact.name,
                kind_json,
                artifact.accepted_revision.map(|revision| revision.to_string()),
            ],
        );
        match result {
            Ok(_) => Ok(()),
            Err(rusqlite::Error::SqliteFailure(error, _))
                if error.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                Err(StoreError::ArtifactAlreadyExists(artifact.id))
            }
            Err(error) => Err(StoreError::Sqlite(error)),
        }
    }

    /// Returns one artifact and its current accepted head.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid durable data or `SQLite` failure.
    pub fn artifact(&self, artifact_id: ArtifactId) -> Result<Option<Artifact>, StoreError> {
        self.connection
            .query_row(
                "SELECT name, kind_json, accepted_revision FROM artifacts WHERE id = ?1",
                [artifact_id.to_string()],
                |row| {
                    let name: String = row.get(0)?;
                    let kind_json: String = row.get(1)?;
                    let accepted_revision: Option<String> = row.get(2)?;
                    Ok((name, kind_json, accepted_revision))
                },
            )
            .optional()?
            .map(|(name, kind_json, accepted_revision)| {
                let kind: ArtifactKind = serde_json::from_str(&kind_json)?;
                let accepted_revision = accepted_revision
                    .map(|value| value.parse())
                    .transpose()
                    .map_err(|_| StoreError::InvalidCommit("stored revision id is invalid"))?;
                Ok(Artifact {
                    id: artifact_id,
                    name,
                    kind,
                    accepted_revision,
                })
            })
            .transpose()
    }

    /// Returns current project metadata and artifacts in stable identity order.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid durable data or `SQLite` failure.
    pub fn snapshot(&self) -> Result<ProjectSnapshot, StoreError> {
        let mut statement = self
            .connection
            .prepare("SELECT id FROM artifacts ORDER BY id")?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let artifacts = ids
            .into_iter()
            .map(|id| {
                let artifact_id = id
                    .parse()
                    .map_err(|_| StoreError::InvalidCommit("stored artifact id is invalid"))?;
                self.artifact(artifact_id)?
                    .ok_or(StoreError::UnknownArtifact(artifact_id))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ProjectSnapshot {
            metadata: self.metadata.clone(),
            artifacts,
        })
    }

    /// Loads the currently accepted revision for one artifact.
    ///
    /// # Errors
    ///
    /// Returns an error when the artifact is unknown or durable revision data is invalid.
    pub fn accepted_revision(
        &self,
        artifact_id: ArtifactId,
    ) -> Result<Option<ArtifactRevision>, StoreError> {
        let artifact = self
            .artifact(artifact_id)?
            .ok_or(StoreError::UnknownArtifact(artifact_id))?;
        artifact
            .accepted_revision
            .map(|revision_id| self.revision(revision_id))
            .transpose()
    }

    /// Loads one persisted creative transformation by stable identity.
    ///
    /// # Errors
    ///
    /// Returns an error when the transformation is absent or its durable JSON is invalid.
    pub fn transformation(
        &self,
        transformation_id: TransformationId,
    ) -> Result<Transformation, StoreError> {
        let transformation_json = self
            .connection
            .query_row(
                "SELECT transformation_json FROM transformations WHERE id = ?1",
                [transformation_id.to_string()],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or(StoreError::UnknownTransformation(transformation_id))?;
        Ok(serde_json::from_str(&transformation_json)?)
    }

    /// Loads one immutable accepted revision by stable identity.
    ///
    /// # Errors
    ///
    /// Returns an error when the revision is absent or its durable JSON is invalid.
    pub fn revision(&self, revision_id: RevisionId) -> Result<ArtifactRevision, StoreError> {
        let revision_json = self
            .connection
            .query_row(
                "SELECT revision_json FROM artifact_revisions WHERE id = ?1",
                [revision_id.to_string()],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .ok_or(StoreError::UnknownRevision(revision_id))?;
        Ok(serde_json::from_str(&revision_json)?)
    }

    /// Reads and verifies exact accepted content bytes.
    ///
    /// # Errors
    ///
    /// Returns an error when the object is absent or fails digest/length verification.
    pub fn read_content(&self, content: &ContentRef) -> Result<Vec<u8>, StoreError> {
        self.objects.read(content)
    }

    /// Publishes candidate bytes and atomically advances an artifact's accepted head.
    ///
    /// Content is durable before the `SQLite` transaction begins. A stale expected
    /// head rejects the transaction without changing accepted history.
    ///
    /// # Errors
    ///
    /// Returns an error for inconsistent provenance, stale heads, or durable write failure.
    pub fn accept(&mut self, commit: AcceptedCommit) -> Result<ArtifactRevision, StoreError> {
        validate_commit(&commit)?;
        let content = self
            .objects
            .publish(&commit.output_bytes, commit.output_media_type)?;
        let created_at_unix_ms = unix_time_ms()?;
        let parents = commit.expected_head.into_iter().collect();
        let revision = ArtifactRevision::new(
            commit.artifact_id,
            parents,
            content,
            commit.transformation.id,
            created_at_unix_ms,
        )?;

        let transaction = self.connection.transaction()?;
        let actual_head: Option<String> = transaction
            .query_row(
                "SELECT accepted_revision FROM artifacts WHERE id = ?1",
                [commit.artifact_id.to_string()],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(StoreError::UnknownArtifact(commit.artifact_id))?;
        let actual_head = actual_head
            .map(|value| value.parse())
            .transpose()
            .map_err(|_| StoreError::InvalidCommit("stored revision id is invalid"))?;
        if actual_head != commit.expected_head {
            return Err(StoreError::RevisionConflict {
                expected: commit.expected_head,
                actual: actual_head,
            });
        }

        transaction.execute(
            "INSERT INTO transformations (id, artifact_id, transformation_json) VALUES (?1, ?2, ?3)",
            params![
                commit.transformation.id.to_string(),
                commit.artifact_id.to_string(),
                serde_json::to_string(&commit.transformation)?,
            ],
        )?;
        transaction.execute(
            "INSERT INTO execution_receipts (attempt_id, transformation_id, receipt_json) VALUES (?1, ?2, ?3)",
            params![
                commit.receipt.attempt_id.to_string(),
                commit.transformation.id.to_string(),
                serde_json::to_string(&commit.receipt)?,
            ],
        )?;
        transaction.execute(
            "INSERT INTO artifact_revisions (id, artifact_id, transformation_id, revision_json) VALUES (?1, ?2, ?3, ?4)",
            params![
                revision.id.to_string(),
                commit.artifact_id.to_string(),
                commit.transformation.id.to_string(),
                serde_json::to_string(&revision)?,
            ],
        )?;
        let changed = transaction.execute(
            "UPDATE artifacts SET accepted_revision = ?1 WHERE id = ?2 AND accepted_revision IS ?3",
            params![
                revision.id.to_string(),
                commit.artifact_id.to_string(),
                commit.expected_head.map(|head| head.to_string()),
            ],
        )?;
        if changed != 1 {
            return Err(StoreError::RevisionConflict {
                expected: commit.expected_head,
                actual: actual_head,
            });
        }
        transaction.commit()?;
        Ok(revision)
    }

    /// Publishes the first accepted revision while atomically creating its artifact.
    ///
    /// Content is durable before the transaction. Artifact identity, transformation,
    /// receipt, revision, and accepted head become visible together or not at all.
    ///
    /// # Errors
    ///
    /// Returns an error for inconsistent provenance, missing inputs, duplicate
    /// artifact identity, or durable write failure.
    pub fn accept_new_artifact(
        &mut self,
        commit: NewArtifactCommit,
    ) -> Result<ArtifactRevision, StoreError> {
        validate_new_artifact_commit(&commit)?;
        let content = self
            .objects
            .publish(&commit.output_bytes, commit.output_media_type)?;
        let revision = ArtifactRevision::new(
            commit.artifact.id,
            Vec::new(),
            content,
            commit.transformation.id,
            unix_time_ms()?,
        )?;
        let kind_json = serde_json::to_string(&commit.artifact.kind)?;
        let transformation_json = serde_json::to_string(&commit.transformation)?;
        let receipt_json = serde_json::to_string(&commit.receipt)?;
        let revision_json = serde_json::to_string(&revision)?;

        let transaction = self.connection.transaction()?;
        for input in &commit.transformation.inputs {
            let exists = transaction
                .query_row(
                    "SELECT 1 FROM artifact_revisions WHERE id = ?1",
                    [input.to_string()],
                    |_| Ok(()),
                )
                .optional()?
                .is_some();
            if !exists {
                return Err(StoreError::UnknownRevision(*input));
            }
        }
        let inserted = transaction.execute(
            "INSERT INTO artifacts (id, name, kind_json, accepted_revision) VALUES (?1, ?2, ?3, ?4)",
            params![
                commit.artifact.id.to_string(),
                commit.artifact.name,
                kind_json,
                revision.id.to_string(),
            ],
        );
        match inserted {
            Ok(_) => {}
            Err(rusqlite::Error::SqliteFailure(error, _))
                if error.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                return Err(StoreError::ArtifactAlreadyExists(commit.artifact.id));
            }
            Err(error) => return Err(StoreError::Sqlite(error)),
        }
        transaction.execute(
            "INSERT INTO transformations (id, artifact_id, transformation_json) VALUES (?1, ?2, ?3)",
            params![
                commit.transformation.id.to_string(),
                commit.artifact.id.to_string(),
                transformation_json,
            ],
        )?;
        transaction.execute(
            "INSERT INTO execution_receipts (attempt_id, transformation_id, receipt_json) VALUES (?1, ?2, ?3)",
            params![
                commit.receipt.attempt_id.to_string(),
                commit.transformation.id.to_string(),
                receipt_json,
            ],
        )?;
        transaction.execute(
            "INSERT INTO artifact_revisions (id, artifact_id, transformation_id, revision_json) VALUES (?1, ?2, ?3, ?4)",
            params![
                revision.id.to_string(),
                commit.artifact.id.to_string(),
                commit.transformation.id.to_string(),
                revision_json,
            ],
        )?;
        transaction.commit()?;
        Ok(revision)
    }
}

fn validate_commit(commit: &AcceptedCommit) -> Result<(), StoreError> {
    if commit.transformation.target_artifact_id != commit.artifact_id {
        return Err(StoreError::InvalidCommit(
            "transformation target does not match artifact",
        ));
    }
    if commit.receipt.transformation_id != commit.transformation.id {
        return Err(StoreError::InvalidCommit(
            "receipt does not belong to transformation",
        ));
    }
    if commit.receipt.outcome != ExecutionOutcome::Succeeded {
        return Err(StoreError::InvalidCommit(
            "only successful execution candidates may be accepted",
        ));
    }
    Ok(())
}

fn validate_new_artifact_commit(commit: &NewArtifactCommit) -> Result<(), StoreError> {
    if commit.artifact.accepted_revision.is_some() {
        return Err(StoreError::InvalidCommit(
            "new artifact must not already have an accepted head",
        ));
    }
    if commit.transformation.target_artifact_id != commit.artifact.id {
        return Err(StoreError::InvalidCommit(
            "transformation target does not match new artifact",
        ));
    }
    if commit.receipt.transformation_id != commit.transformation.id {
        return Err(StoreError::InvalidCommit(
            "receipt does not belong to transformation",
        ));
    }
    if commit.receipt.outcome != ExecutionOutcome::Succeeded {
        return Err(StoreError::InvalidCommit(
            "only successful execution candidates may be accepted",
        ));
    }
    Ok(())
}

fn write_manifest(root: &Path, metadata: &ProjectMetadata) -> Result<(), StoreError> {
    let manifest = BundleManifest {
        schema: MANIFEST_SCHEMA.to_owned(),
        metadata: metadata.clone(),
    };
    let bytes = serde_json::to_vec_pretty(&manifest)?;
    let temporary = root.join(format!(".{MANIFEST_FILE}-{}.tmp", Uuid::now_v7()));
    let destination = root.join(MANIFEST_FILE);
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, destination)?;
        Ok::<(), StoreError>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn unix_time_ms() -> Result<u64, StoreError> {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| StoreError::IntegerOutOfRange)?
        .as_millis();
    u64::try_from(milliseconds).map_err(|_| StoreError::IntegerOutOfRange)
}

#[cfg(test)]
mod tests;
