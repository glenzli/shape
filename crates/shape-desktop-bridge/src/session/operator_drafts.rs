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
use crate::operator_catalog::{
    AUDIO_SPEECH_OPERATOR, TEXT_TRANSFORM_OPERATOR, configuration_for_audio_speech,
    configuration_for_mode_and_instruction, default_audio_speech_configuration,
    validate_draft_configuration,
};

/// Identity-addressed drafts owned by one open desktop session.
#[derive(Debug, Clone, Default)]
pub(crate) struct OperatorDrafts {
    graphs: Vec<ArtifactWorkingGraph>,
}

impl OperatorDrafts {
    pub(crate) fn from_graphs(graphs: Vec<ArtifactWorkingGraph>) -> Result<Self, String> {
        for graph in &graphs {
            graph.validate().map_err(|error| error.to_string())?;
            for draft in graph.operators() {
                validate_draft_configuration(draft)?;
            }
        }
        Ok(Self { graphs })
    }

    /// Upgrades legacy preset speech drafts that predate Operator-owned
    /// configuration. The caller persists every returned graph before making
    /// the session available, so execution never observes UI-only defaults.
    pub(crate) fn initialize_audio_speech_defaults(
        &mut self,
    ) -> Result<Vec<ArtifactWorkingGraph>, String> {
        let mut changed = Vec::new();
        for graph in &mut self.graphs {
            let draft_ids = graph
                .operators()
                .iter()
                .filter(|draft| {
                    draft.operator_type().as_str() == AUDIO_SPEECH_OPERATOR
                        && draft.configuration().is_none()
                })
                .map(|draft| draft.id().clone())
                .collect::<Vec<_>>();
            if draft_ids.is_empty() {
                continue;
            }
            for draft_id in draft_ids {
                if !graph.set_operator_configuration(
                    &draft_id,
                    Some(default_audio_speech_configuration()?),
                ) {
                    return Err("Operator draft disappeared during legacy configuration".to_owned());
                }
            }
            graph.validate().map_err(|error| error.to_string())?;
            changed.push(graph.clone());
        }
        Ok(changed)
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
        let draft = graph
            .add_operator(
                OperatorTypeId::new(descriptor.type_key).map_err(|error| error.to_string())?,
                OperatorDataTypeId::new(descriptor.input_data_type)
                    .map_err(|error| error.to_string())?,
                OperatorDataTypeId::new(descriptor.output_data_type)
                    .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())?;
        if descriptor.type_key == AUDIO_SPEECH_OPERATOR && draft.configuration().is_none() {
            let configuration = default_audio_speech_configuration()?;
            if !graph.set_operator_configuration(draft.id(), Some(configuration)) {
                return Err("Operator draft disappeared during default configuration".to_owned());
            }
            return graph
                .operators()
                .iter()
                .find(|operator| operator.id() == draft.id())
                .cloned()
                .ok_or_else(|| {
                    "Operator draft disappeared during default configuration".to_owned()
                });
        }
        Ok(draft)
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

    pub(crate) fn update_text_transform_configuration(
        &mut self,
        draft_id: &str,
        mode_key: &str,
        instruction: &str,
    ) -> Result<(ArtifactId, WorkingOperatorDraft), String> {
        let draft_id = OperatorNodeId::new(draft_id).map_err(|error| error.to_string())?;
        let Some(graph_index) = self.graph_index_for_draft(&draft_id) else {
            return Err("Operator draft does not exist".to_owned());
        };
        let operator_type = self.graphs[graph_index]
            .operators()
            .iter()
            .find(|draft| draft.id() == &draft_id)
            .expect("located draft remains in its Working Graph")
            .operator_type()
            .as_str();
        if operator_type != TEXT_TRANSFORM_OPERATOR {
            return Err("draft is not a text.transform Operator".to_owned());
        }
        let configuration = configuration_for_mode_and_instruction(mode_key, instruction)?;
        let artifact_id = self.graphs[graph_index].context_artifact_id();
        if !self.graphs[graph_index].set_operator_configuration(&draft_id, configuration) {
            return Err("Operator draft disappeared during configuration".to_owned());
        }
        let draft = self.graphs[graph_index]
            .operators()
            .iter()
            .find(|draft| draft.id() == &draft_id)
            .expect("configured draft remains in its Working Graph")
            .clone();
        Ok((artifact_id, draft))
    }

    pub(crate) fn update_audio_speech_configuration(
        &mut self,
        draft_id: &str,
        preset_alias: &str,
        preset_catalog_revision: &str,
        language: &str,
        speed_milli: u16,
        synthetic_disclosure_required: bool,
    ) -> Result<(ArtifactId, WorkingOperatorDraft), String> {
        let draft_id = OperatorNodeId::new(draft_id).map_err(|error| error.to_string())?;
        let Some(graph_index) = self.graph_index_for_draft(&draft_id) else {
            return Err("Operator draft does not exist".to_owned());
        };
        let operator_type = self.graphs[graph_index]
            .operators()
            .iter()
            .find(|draft| draft.id() == &draft_id)
            .expect("located draft remains in its Working Graph")
            .operator_type()
            .as_str();
        if operator_type != AUDIO_SPEECH_OPERATOR {
            return Err("draft is not an audio.speech_synthesize Operator".to_owned());
        }
        let configuration = configuration_for_audio_speech(
            preset_alias,
            preset_catalog_revision,
            language,
            speed_milli,
            synthetic_disclosure_required,
        )?;
        let artifact_id = self.graphs[graph_index].context_artifact_id();
        if !self.graphs[graph_index].set_operator_configuration(&draft_id, Some(configuration)) {
            return Err("Operator draft disappeared during configuration".to_owned());
        }
        let draft = self.graphs[graph_index]
            .operators()
            .iter()
            .find(|draft| draft.id() == &draft_id)
            .expect("configured draft remains in its Working Graph")
            .clone();
        Ok((artifact_id, draft))
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

    fn graph_index_for_draft(&self, draft_id: &OperatorNodeId) -> Option<usize> {
        self.graphs
            .iter()
            .position(|graph| graph.operators().iter().any(|draft| draft.id() == draft_id))
    }
}

#[cfg(test)]
mod tests;
