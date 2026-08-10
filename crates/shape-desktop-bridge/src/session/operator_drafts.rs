//! Project-backed Operator drafts before a real Candidate exists.
//!
//! A draft makes the graph authoring entry visible without pretending that an
//! unexecuted Operator is already an accepted `SceneRevision`. The first real
//! Candidate replaces the matching draft; acceptance continues through the
//! existing immutable Artifact history boundary.

use shape_domain::{
    Artifact, ArtifactId, ArtifactWorkingGraph, OperatorDataTypeId, OperatorNodeId, OperatorTypeId,
    WorkingOperatorDraft,
};

use crate::operator_catalog::OperatorDescriptor;

/// Identity-addressed drafts owned by one open desktop session.
#[derive(Debug, Clone, Default)]
pub(crate) struct OperatorDrafts {
    graphs: Vec<ArtifactWorkingGraph>,
}

impl OperatorDrafts {
    pub(crate) fn from_graphs(graphs: Vec<ArtifactWorkingGraph>) -> Result<Self, String> {
        for graph in &graphs {
            graph.validate().map_err(|error| error.to_string())?;
        }
        Ok(Self { graphs })
    }

    /// Begins or reuses one compatible Operator draft.
    pub(crate) fn begin(
        &mut self,
        artifact: &Artifact,
        descriptor: &OperatorDescriptor,
    ) -> Result<WorkingOperatorDraft, String> {
        let expected_revision = artifact
            .accepted_revision
            .ok_or_else(|| "an Operator draft requires an accepted source".to_owned())?;
        let graph = if let Some(index) = self
            .graphs
            .iter()
            .position(|graph| graph.context_artifact_id() == artifact.id)
        {
            if self.graphs[index].expected_revision_id() != expected_revision {
                return Err("the Working Graph is based on a stale accepted source".to_owned());
            }
            &mut self.graphs[index]
        } else {
            self.graphs
                .push(ArtifactWorkingGraph::new(artifact.id, expected_revision));
            self.graphs
                .last_mut()
                .expect("a just-pushed Working Graph exists")
        };
        graph
            .add_operator(
                OperatorTypeId::new(descriptor.type_key).map_err(|error| error.to_string())?,
                OperatorDataTypeId::new(descriptor.input_data_type)
                    .map_err(|error| error.to_string())?,
                OperatorDataTypeId::new(descriptor.output_data_type)
                    .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())
    }

    pub(crate) fn entries(&self) -> impl Iterator<Item = (ArtifactId, &WorkingOperatorDraft)> {
        self.graphs.iter().flat_map(|graph| {
            graph
                .operators()
                .iter()
                .map(|operator| (graph.context_artifact_id(), operator))
        })
    }

    pub(crate) fn graph(&self, artifact_id: ArtifactId) -> Option<&ArtifactWorkingGraph> {
        self.graphs
            .iter()
            .find(|graph| graph.context_artifact_id() == artifact_id)
    }

    pub(crate) fn discard(&mut self, draft_id: &str) -> Result<ArtifactId, String> {
        let draft_id = OperatorNodeId::new(draft_id).map_err(|error| error.to_string())?;
        let Some(index) = self
            .graphs
            .iter_mut()
            .position(|graph| graph.remove_operator(&draft_id))
        else {
            return Err("Operator draft does not exist".to_owned());
        };
        let artifact_id = self.graphs[index].context_artifact_id();
        if self.graphs[index].is_empty() {
            self.graphs.remove(index);
        }
        Ok(artifact_id)
    }

    pub(crate) fn finish(&mut self, artifact_id: ArtifactId, operator_type: &str) -> bool {
        let Some(index) = self
            .graphs
            .iter()
            .position(|graph| graph.context_artifact_id() == artifact_id)
        else {
            return false;
        };
        let changed = self.graphs[index].remove_operator_type(operator_type);
        if self.graphs[index].is_empty() {
            self.graphs.remove(index);
        }
        changed
    }

    pub(crate) fn clear_artifact(&mut self, artifact_id: ArtifactId) -> bool {
        let before = self.graphs.len();
        self.graphs
            .retain(|graph| graph.context_artifact_id() != artifact_id);
        self.graphs.len() != before
    }
}

#[cfg(test)]
mod tests;
