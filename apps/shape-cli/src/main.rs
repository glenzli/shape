//! Runnable public consumer for the first Shape vertical slice.

use std::{
    env,
    error::Error,
    io,
    path::{Path, PathBuf},
};

use shape_core::ShapeProject;
use shape_domain::{ArtifactKind, Constraint, ConstraintKind, ConstraintStrength, IntentSpec};

fn main() {
    if let Err(error) = run() {
        eprintln!("shape-cli: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args().skip(1);
    let command = arguments.next().ok_or_else(usage_error)?;
    let path = PathBuf::from(arguments.next().ok_or_else(usage_error)?);
    if arguments.next().is_some() {
        return Err(usage_error().into());
    }

    match command.as_str() {
        "demo" => demo(&path),
        "inspect" => inspect(&path),
        _ => Err(usage_error().into()),
    }
}

fn demo(path: &Path) -> Result<(), Box<dyn Error>> {
    let mut project = ShapeProject::create(path, "Shape Foundation Demo")?;
    let artifact = project.create_artifact("Story", ArtifactKind::TextDocument)?;

    let initial = project.propose_text(
        artifact.id,
        None,
        "A summer afternoon.",
        IntentSpec::new("Import the initial line")?,
        Vec::new(),
    )?;
    println!("candidate 1: {}", initial.text());
    let first = project.accept_text(initial)?;

    let revised = project.propose_text(
        artifact.id,
        Some(first.id),
        "A quiet summer afternoon.",
        IntentSpec::new("Make the atmosphere quiet")?,
        vec![Constraint::new(
            ConstraintKind::PreserveContent,
            ConstraintStrength::Hard,
            "Keep the summer afternoon subject",
            None,
        )?],
    )?;
    println!("candidate 2: {}", revised.text());
    let second = project.accept_text(revised)?;

    println!("project: {}", path.display());
    println!("artifact: {}", artifact.id);
    println!("accepted revisions: {} -> {}", first.id, second.id);
    Ok(())
}

fn inspect(path: &Path) -> Result<(), Box<dyn Error>> {
    let project = ShapeProject::open(path)?;
    let snapshot = project.snapshot()?;
    println!(
        "project: {} ({})",
        snapshot.metadata.name, snapshot.metadata.id
    );
    println!("schema: {}", snapshot.metadata.schema_revision);
    for artifact in snapshot.artifacts {
        println!(
            "artifact: {} {:?} head={:?}",
            artifact.name, artifact.kind, artifact.accepted_revision
        );
        if let Some(content) = project.read_accepted(artifact.id)? {
            println!("  digest: {}", content.revision.content.digest);
            if content.revision.content.media_type.starts_with("text/") {
                println!("  text: {}", String::from_utf8_lossy(&content.bytes));
            }
        }
    }
    Ok(())
}

fn usage_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "usage: shape-cli <demo|inspect> <project.shape>",
    )
}
