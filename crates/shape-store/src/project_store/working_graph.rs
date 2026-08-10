//! Mutable compatibility-Scene Working Graph persistence.

use rusqlite::{OptionalExtension, params};
use shape_domain::{ArtifactId, ArtifactWorkingGraph};

use super::ProjectStore;
use crate::StoreError;

impl ProjectStore {
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
            if artifact.accepted_revision == Some(graph.expected_revision_id()) {
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
        let expected = Some(graph.expected_revision_id());
        let changed = self.connection.execute(
            "INSERT INTO artifact_working_graphs (artifact_id, graph_json)
             SELECT ?1, ?2
             WHERE (SELECT accepted_revision FROM artifacts WHERE id = ?1) = ?3
             ON CONFLICT(artifact_id) DO UPDATE SET graph_json = excluded.graph_json",
            params![
                graph.context_artifact_id().to_string(),
                serde_json::to_string(graph)?,
                graph.expected_revision_id().to_string(),
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

#[cfg(test)]
mod tests;
