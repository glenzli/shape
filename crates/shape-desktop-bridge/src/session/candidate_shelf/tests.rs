use std::{fs, path::PathBuf};

use shape_core::ShapeProject;
use shape_domain::{ArtifactKind, IntentSpec};
use uuid::Uuid;

use super::{Candidate, CandidateShelf};

fn test_root() -> PathBuf {
    std::env::temp_dir().join(format!("shape-candidate-shelf-{}", Uuid::now_v7()))
}

fn text_candidate(
    project: &ShapeProject,
    artifact_id: shape_domain::ArtifactId,
    text: &str,
) -> Candidate {
    Candidate::Text(
        project
            .propose_text(
                artifact_id,
                None,
                text,
                IntentSpec::new("Explore text alternative").expect("intent valid"),
                Vec::new(),
            )
            .expect("candidate executes"),
    )
}

#[test]
fn shelf_projects_newest_first_and_discards_exact_identity() {
    let root = test_root();
    let project = ShapeProject::create(&root, "Candidate Shelf").expect("project creates");
    let artifact = project
        .create_artifact("Story", ArtifactKind::TextDocument)
        .expect("artifact creates");
    let first = text_candidate(&project, artifact.id, "First");
    let first_id = first.id();
    let second = text_candidate(&project, artifact.id, "Second");
    let second_id = second.id();
    let mut shelf = CandidateShelf::default();
    shelf.push(first);
    shelf.push(second);

    assert!(shelf.contains_text(artifact.id, "First"));
    assert!(!shelf.contains_text(artifact.id, "Unseen"));
    assert_eq!(
        shelf.newest_first().map(Candidate::id).collect::<Vec<_>>(),
        vec![second_id.clone(), first_id.clone()]
    );
    shelf.discard(&first_id).expect("first candidate discards");
    assert_eq!(
        shelf.newest_first().map(Candidate::id).collect::<Vec<_>>(),
        vec![second_id]
    );
    assert!(shelf.discard("missing-candidate").is_err());
    fs::remove_dir_all(root).expect("test project removes");
}

#[test]
fn accepting_policy_can_clear_only_one_artifacts_candidates() {
    let root = test_root();
    let project = ShapeProject::create(&root, "Candidate Shelf").expect("project creates");
    let story = project
        .create_artifact("Story", ArtifactKind::TextDocument)
        .expect("story creates");
    let notes = project
        .create_artifact("Notes", ArtifactKind::TextDocument)
        .expect("notes create");
    let mut shelf = CandidateShelf::default();
    shelf.push(text_candidate(&project, story.id, "Story option one"));
    shelf.push(text_candidate(&project, notes.id, "Note option"));
    shelf.push(text_candidate(&project, story.id, "Story option two"));

    shelf.discard_artifact(story.id);

    let remaining = shelf.newest_first().collect::<Vec<_>>();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].artifact_id(), notes.id);
    fs::remove_dir_all(root).expect("test project removes");
}
