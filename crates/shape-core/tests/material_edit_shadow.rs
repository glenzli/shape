//! Actual Shadow external-stage roundtrip; opt in with `SHAPE_SHADOW_BINARY` and `SHAPE_SHADOW_EVIDENCE`.
use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};
use shape_core::ShapeProject;
use shape_domain::{ContentDigest, MaterialEditor, WorkingInput};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn run_shadow(binary: &Path, directory: &Path, evidence: &Path, exposure: &str) {
    let output = Command::new("python3")
        .arg("-B")
        .arg(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scripts/test_material_shadow.py"),
        )
        .arg("--binary")
        .arg(binary)
        .arg("--request")
        .arg(directory.join("shadow-request.json"))
        .arg("--result")
        .arg(directory.join("shadow-result.json"))
        .arg("--evidence")
        .arg(evidence)
        .arg("--exposure")
        .arg(exposure)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore = "requires separately built Shadow; isolated synthetic pixels only"]
fn real_shadow_completed_return_accepts_separate_versions_and_refreshes_downstream() {
    let binary = PathBuf::from(std::env::var_os("SHAPE_SHADOW_BINARY").expect("Shadow binary"));
    let root =
        PathBuf::from(std::env::var_os("SHAPE_SHADOW_EVIDENCE").expect("new evidence directory"));
    fs::create_dir(&root).unwrap();
    let png = synthetic_png();
    let mut project =
        ShapeProject::create(root.join("fixture.shape"), "Shadow integration").unwrap();
    let original = project.import_raster_bytes(png, "Original").unwrap();
    let original_bytes = project
        .read_accepted(original.artifact_id)
        .unwrap()
        .unwrap()
        .bytes;
    let ticket = project
        .create_material_edit(
            WorkingInput {
                artifact_id: original.artifact_id,
                revision_id: original.id,
            },
            "Shadow edited",
            MaterialEditor::Shadow,
        )
        .unwrap();
    let mut accepted = Vec::new();
    let mut downstream = None;
    for (index, exposure) in ["0.75", "-0.75"].into_iter().enumerate() {
        let current = project
            .material_edit_ticket(ticket.output_artifact_id)
            .unwrap();
        let directory = root.join(format!("handoff-{index}"));
        let handoff = project.export_material_edit(&current, &directory).unwrap();
        run_shadow(
            &binary,
            &directory,
            &root.join(format!("shadow-{index}")),
            exposure,
        );
        let result = directory.join("shadow-result.json");
        let candidate = project
            .propose_shadow_return(&handoff, &result, None)
            .unwrap();
        assert_eq!(
            candidate.returned_digest(),
            ContentDigest::from_bytes(&fs::read(directory.join("edited.png")).unwrap())
        );
        let revision = project.accept_material_return(candidate).unwrap();
        assert!(
            project
                .propose_shadow_return(&handoff, &result, None)
                .is_err()
        );
        if index == 0 {
            downstream = Some(
                project
                    .create_material_edit(
                        WorkingInput {
                            artifact_id: revision.artifact_id,
                            revision_id: revision.id,
                        },
                        "Downstream",
                        MaterialEditor::Shadow,
                    )
                    .unwrap(),
            );
        }
        accepted.push(revision);
    }
    let downstream = downstream.unwrap();
    assert!(project.material_edit_source(&downstream).is_err());
    assert_eq!(
        downstream.draft.input().unwrap().revision_id,
        accepted[0].id
    );
    let refreshed = project.restart_material_edit(&downstream, true).unwrap();
    assert_eq!(refreshed.draft.input().unwrap().revision_id, accepted[1].id);
    assert_eq!(
        project
            .read_accepted(original.artifact_id)
            .unwrap()
            .unwrap()
            .bytes,
        original_bytes
    );
    drop(project);
    let project = ShapeProject::open(root.join("fixture.shape")).unwrap();
    assert_eq!(
        project
            .material_edit_ticket(ticket.output_artifact_id)
            .unwrap()
            .expected_output_head,
        Some(accepted[1].id)
    );
    fs::write(root.join("shape-acceptance.json"),serde_json::to_vec_pretty(&serde_json::json!({
        "original":original,"accepted":accepted,"downstream_before":downstream,"downstream_after":refreshed,
        "completed_receipts_verified":true,"original_bytes_unchanged":true,"reopened":true
    })).unwrap()).unwrap();
}

fn synthetic_png() -> Vec<u8> {
    let mut pixels = Vec::new();
    for y in 0..426_u32 {
        for x in 0..640_u32 {
            pixels.extend([
                u8::try_from(20 + x * 130 / 640).unwrap(),
                u8::try_from(24 + y * 115 / 426).unwrap(),
                70,
                255,
            ]);
        }
    }
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&pixels, 640, 426, ColorType::Rgba8.into())
        .unwrap();
    png
}
