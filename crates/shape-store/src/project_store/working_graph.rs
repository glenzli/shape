//! Mutable producer graphs, reserved outputs and atomic acceptance coordination.

use rusqlite::{OptionalExtension, params};
use shape_domain::{Artifact, ArtifactId, ArtifactWorkingGraph};

use super::ProjectStore;
use crate::StoreError;

impl ProjectStore {
    /// Removes a node graph and its empty reserved output atomically.
    /// Accepted output material is retained.
    /// # Errors
    /// Returns storage errors without changing accepted history.
    pub fn discard_output_working_graph(
        &mut self,
        artifact_id: ArtifactId,
    ) -> Result<(), StoreError> {
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "DELETE FROM artifact_working_graphs WHERE artifact_id = ?1",
            [artifact_id.to_string()],
        )?;
        transaction.execute(
            "DELETE FROM artifacts WHERE id = ?1 AND accepted_revision IS NULL",
            [artifact_id.to_string()],
        )?;
        transaction.commit()?;
        Ok(())
    }
    /// Loads one persisted compatibility-Scene Working Graph.
    ///
    /// # Errors
    ///
    /// Returns an error when storage, JSON, identity, or domain validation fails.
    pub fn artifact_working_graph(
        &self,
        artifact_id: ArtifactId,
    ) -> Result<Option<ArtifactWorkingGraph>, StoreError> {
        self.connection
            .query_row(
                "SELECT graph_json FROM artifact_working_graphs WHERE artifact_id = ?1",
                [artifact_id.to_string()],
                |row| row.get::<_, String>(0),
            )
            .optional()?
            .map(|graph_json| {
                let graph: ArtifactWorkingGraph = serde_json::from_str(&graph_json)?;
                graph.validate()?;
                if graph.context_artifact_id() != artifact_id {
                    return Err(StoreError::InvalidCommit(
                        "working graph identity does not match its row",
                    ));
                }
                Ok(graph)
            })
            .transpose()
    }

    /// Loads every current-head Working Graph in stable Artifact identity order.
    ///
    /// A stale mutable graph is ignored rather than allowed back into an editor
    /// after another session has advanced immutable accepted history.
    ///
    /// # Errors
    ///
    /// Returns an error when storage or any current graph fails validation.
    pub fn artifact_working_graphs(&self) -> Result<Vec<ArtifactWorkingGraph>, StoreError> {
        let mut statement = self
            .connection
            .prepare("SELECT artifact_id FROM artifact_working_graphs ORDER BY artifact_id")?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let mut graphs = Vec::with_capacity(ids.len());
        for id in ids {
            let artifact_id = id.parse().map_err(|_| {
                StoreError::InvalidCommit("working graph Artifact identity is invalid")
            })?;
            let graph =
                self.artifact_working_graph(artifact_id)?
                    .ok_or(StoreError::InvalidCommit(
                        "working graph disappeared during its snapshot",
                    ))?;
            let artifact = self
                .artifact(artifact_id)?
                .ok_or(StoreError::UnknownArtifact(artifact_id))?;
            if artifact.accepted_revision == graph.expected_revision_id() {
                graphs.push(graph);
            }
        }
        Ok(graphs)
    }

    /// Creates or replaces one non-empty Working Graph against its exact input head.
    ///
    /// # Errors
    ///
    /// Returns an error for an empty or malformed graph, an unknown Artifact,
    /// a stale expected head, serialization failure, or storage failure.
    pub fn save_artifact_working_graph(
        &self,
        graph: &ArtifactWorkingGraph,
    ) -> Result<(), StoreError> {
        graph.validate()?;
        if graph.is_empty() {
            return Err(StoreError::InvalidCommit(
                "empty working graphs must be deleted instead of persisted",
            ));
        }
        let expected = graph.expected_revision_id();
        let changed = self.connection.execute(
            "INSERT INTO artifact_working_graphs (artifact_id, graph_json)
             SELECT ?1, ?2
             WHERE (SELECT accepted_revision FROM artifacts WHERE id = ?1) IS ?3
             ON CONFLICT(artifact_id) DO UPDATE SET graph_json = excluded.graph_json",
            params![
                graph.context_artifact_id().to_string(),
                serde_json::to_string(graph)?,
                graph
                    .expected_revision_id()
                    .map(|revision| revision.to_string()),
            ],
        )?;
        if changed == 0 {
            let artifact = self
                .artifact(graph.context_artifact_id())?
                .ok_or(StoreError::UnknownArtifact(graph.context_artifact_id()))?;
            return Err(StoreError::RevisionConflict {
                expected,
                actual: artifact.accepted_revision,
            });
        }
        Ok(())
    }

    /// Atomically reserves an output Artifact and its producer graph.
    ///
    /// # Errors
    ///
    /// Rejects accepted Artifacts, mismatched identities, accepted-head graphs,
    /// empty or malformed graphs, duplicate identities, and storage failures.
    pub fn insert_source_artifact_with_working_graph(
        &mut self,
        artifact: &Artifact,
        graph: &ArtifactWorkingGraph,
    ) -> Result<(), StoreError> {
        graph.validate()?;
        if artifact.accepted_revision.is_some()
            || graph.expected_revision_id().is_some()
            || graph.context_artifact_id() != artifact.id
            || graph.is_empty()
        {
            return Err(StoreError::InvalidCommit(
                "source draft must target the same unaccepted Artifact",
            ));
        }
        let kind_json = serde_json::to_string(&artifact.kind)?;
        let graph_json = serde_json::to_string(graph)?;
        let transaction = self.connection.transaction()?;
        let inserted = transaction.execute(
            "INSERT INTO artifacts (id, name, kind_json, accepted_revision)
             VALUES (?1, ?2, ?3, NULL)",
            params![artifact.id.to_string(), artifact.name, kind_json],
        );
        match inserted {
            Ok(_) => {}
            Err(rusqlite::Error::SqliteFailure(error, _))
                if error.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                return Err(StoreError::ArtifactAlreadyExists(artifact.id));
            }
            Err(error) => return Err(StoreError::Sqlite(error)),
        }
        transaction.execute(
            "INSERT INTO artifact_working_graphs (artifact_id, graph_json) VALUES (?1, ?2)",
            params![artifact.id.to_string(), graph_json],
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// Removes mutable editor state without changing accepted history.
    ///
    /// # Errors
    ///
    /// Returns an error when the storage mutation fails.
    pub fn delete_artifact_working_graph(&self, artifact_id: ArtifactId) -> Result<(), StoreError> {
        self.connection.execute(
            "DELETE FROM artifact_working_graphs WHERE artifact_id = ?1",
            [artifact_id.to_string()],
        )?;
        Ok(())
    }
}

pub(super) fn acceptance_graph(
    transaction: &rusqlite::Transaction<'_>,
    artifact_id: ArtifactId,
    expected_head: Option<shape_domain::RevisionId>,
    expected_node: Option<&shape_domain::WorkingOperatorDraft>,
) -> Result<Option<ArtifactWorkingGraph>, StoreError> {
    let graph_json: Option<String> = transaction
        .query_row(
            "SELECT graph_json FROM artifact_working_graphs WHERE artifact_id = ?1",
            [artifact_id.to_string()],
            |row| row.get(0),
        )
        .optional()?;
    let graph = graph_json
        .map(|json| serde_json::from_str::<shape_domain::ArtifactWorkingGraph>(&json))
        .transpose()?;
    if let Some(node) = expected_node
        && !graph.as_ref().is_some_and(|g| {
            g.expected_revision_id() == expected_head && g.operators().contains(node)
        })
    {
        return Err(StoreError::InvalidCommit(
            "the authored node changed after execution",
        ));
    }
    Ok(graph)
}

#[cfg(test)]
mod tests;
