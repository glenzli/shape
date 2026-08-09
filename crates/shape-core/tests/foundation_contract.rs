use std::{fs, path::PathBuf};

use shape_core::ShapeProject;
use shape_domain::{ArtifactKind, Constraint, ConstraintKind, ConstraintStrength, IntentSpec};
use uuid::Uuid;

fn test_root() -> PathBuf {
    std::env::temp_dir().join(format!("shape-core-e2e-{}", Uuid::now_v7()))
}

#[test]
fn candidate_does_not_advance_history_until_acceptance() {
    let root = test_root();
    let mut project = ShapeProject::create(&root, "Foundation").expect("project creates");
    let artifact = project
        .create_artifact("Story", ArtifactKind::TextDocument)
        .expect("artifact creates");
    let candidate = project
        .propose_text(
            artifact.id,
            None,
            "A quiet summer afternoon.",
            IntentSpec::new("Import the opening line").expect("intent valid"),
            vec![
                Constraint::new(
                    ConstraintKind::PreserveContent,
                    ConstraintStrength::Hard,
                    "Retain the subject",
                    None,
                )
                .expect("constraint valid"),
            ],
        )
        .expect("candidate executes");

    assert_eq!(candidate.text(), "A quiet summer afternoon.");
    assert!(
        project
            .read_accepted(artifact.id)
            .expect("head reads")
            .is_none()
    );

    let accepted = project.accept_text(candidate).expect("candidate accepts");
    drop(project);
    let reopened = ShapeProject::open(&root).expect("project reopens");
    let content = reopened
        .read_accepted(artifact.id)
        .expect("head reads")
        .expect("accepted content exists");
    assert_eq!(content.revision.id, accepted.id);
    assert_eq!(content.bytes, b"A quiet summer afternoon.");
    fs::remove_dir_all(&root).expect("test bundle removes");
}
