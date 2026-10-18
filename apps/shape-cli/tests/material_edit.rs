use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};
use serde_json::{Value, json};
use shape_core::ShapeProject;
use shape_domain::{ArtifactRevision, ContentDigest, WorkingInput};
use std::{fs, path::Path, process::Command};
fn png(c: u8) -> Vec<u8> {
    let mut b = Vec::new();
    PngEncoder::new(&mut b)
        .write_image(&[c, 20, 30, 255], 1, 1, ColorType::Rgba8.into())
        .unwrap();
    b
}
fn run(root: &Path, project: &Path, request: &Value, success: bool) -> Value {
    run_project(root, project, "material-edit", request, success)
}
fn run_project(
    root: &Path,
    project: &Path,
    command: &str,
    request: &Value,
    success: bool,
) -> Value {
    let file = root.join("request.json");
    fs::write(&file, serde_json::to_vec(request).unwrap()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_shape-cli"))
        .arg(command)
        .arg(project)
        .arg(file)
        .output()
        .unwrap();
    assert_eq!(
        output.status.success(),
        success,
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    let schema = if command == "material-edit" {
        "shape.material-edit-result/1"
    } else {
        "shape.picture-book-result/1"
    };
    assert_eq!(value["schema"], schema);
    assert_eq!(value["status"], if success { "ok" } else { "error" });
    if !success {
        assert!(value.get("result").is_none());
    }
    value["result"].clone()
}

#[test]
fn cli_roundtrip_is_review_then_explicit_digest_pinned_accept_and_duplicate_is_rejected() {
    let root = std::env::temp_dir().join(format!("shape-material-cli-{}", uuid::Uuid::now_v7()));
    fs::create_dir(&root).unwrap();
    let path = root.join("fixture.shape");
    let mut project = ShapeProject::create(&path, "CLI").unwrap();
    let source = project.import_raster_bytes(png(10), "Source").unwrap();
    let begin = run(
        &root,
        &path,
        &json!({"action":"begin","source":{"artifact_id":source.artifact_id,"revision_id":source.id},"name":"Edited","editor":"shadow"}),
        true,
    );
    let ticket = &begin["ticket"];
    run(
        &root,
        &path,
        &json!({"action":"export","ticket":ticket,"destination":root.join("handoff")}),
        true,
    );
    run(
        &root,
        &path,
        &json!({"action":"export","ticket":ticket,"destination":root.join("handoff")}),
        false,
    );
    let returned = root.join("edited.png");
    fs::write(&returned, png(80)).unwrap();
    let handoff = root.join("handoff/handoff.json");
    let review = run(
        &root,
        &path,
        &json!({"action":"review","handoff":handoff,"returned_file":returned}),
        true,
    );
    assert_eq!(review["accepted"], false);
    let artifact = serde_json::from_value(ticket["output_artifact_id"].clone()).unwrap();
    assert!(project.read_accepted(artifact).unwrap().is_none());
    let accept = json!({"action":"accept","handoff":handoff,"returned_file":returned,"returned_blake3":review["returned_blake3"]});
    fs::write(&returned, png(81)).unwrap();
    run(&root, &path, &accept, false);
    fs::write(&returned, png(80)).unwrap();
    run(&root, &path, &accept, true);
    run(&root, &path, &accept, false);
    assert_eq!(
        project.read_accepted(artifact).unwrap().unwrap().bytes,
        png(80)
    );
    assert_eq!(
        project
            .read_accepted(source.artifact_id)
            .unwrap()
            .unwrap()
            .revision
            .id,
        source.id
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn unknown_action_fields_and_cancelled_handoffs_cannot_mutate_accepted_heads() {
    let root = std::env::temp_dir().join(format!(
        "shape-material-cli-cancel-{}",
        uuid::Uuid::now_v7()
    ));
    fs::create_dir(&root).unwrap();
    let path = root.join("fixture.shape");
    let mut project = ShapeProject::create(&path, "CLI").unwrap();
    let source = project.import_raster_bytes(png(1), "Source").unwrap();
    let req = json!({"action":"begin","source":{"artifact_id":source.artifact_id,"revision_id":source.id},"name":"Edited","editor":"shadow"});
    let mut unknown = req.clone();
    unknown["surprise"] = json!(true);
    run(&root, &path, &unknown, false);
    assert_eq!(project.snapshot().unwrap().artifacts.len(), 1);
    let ticket = run(&root, &path, &req, true)["ticket"].clone();
    run(
        &root,
        &path,
        &json!({"action":"export","ticket":ticket,"destination":root.join("handoff")}),
        true,
    );
    run(
        &root,
        &path,
        &json!({"action":"cancel","ticket":ticket}),
        true,
    );
    let returned = root.join("edited.png");
    fs::write(&returned, png(2)).unwrap();
    run(
        &root,
        &path,
        &json!({"action":"review","handoff":root.join("handoff/handoff.json"),"returned_file":returned}),
        false,
    );
    assert!(
        project
            .read_accepted(serde_json::from_value(ticket["output_artifact_id"].clone()).unwrap())
            .unwrap()
            .is_none()
    );
    fs::remove_dir_all(root).unwrap();
}

struct Fixture {
    root: std::path::PathBuf,
    path: std::path::PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root =
            std::env::temp_dir().join(format!("shape-resumed-material-{}", uuid::Uuid::now_v7()));
        fs::create_dir(&root).unwrap();
        let path = root.join("fixture.shape");
        Self { root, path }
    }
    fn material(&self, request: &Value, success: bool) -> Value {
        run(&self.root, &self.path, request, success)
    }
    fn book(&self, request: &Value) -> Value {
        run_project(&self.root, &self.path, "picture-book", request, true)
    }
    fn accept_return(&self, ticket: &Value, bytes: &[u8], label: &str, convert: bool) -> Value {
        let directory = self.root.join(label);
        self.material(
            &json!({"action":"export","ticket":ticket,"destination":directory}),
            true,
        );
        let returned = self.root.join(format!("{label}.returned"));
        fs::write(&returned, bytes).unwrap();
        let handoff = directory.join("handoff.json");
        let review = self.material(&json!({"action":"review","handoff":handoff,"returned_file":returned,"allow_pcm24_conversion":convert}), true);
        self.material(&json!({"action":"accept","handoff":handoff,"returned_file":returned,"returned_blake3":review["returned_blake3"],"allow_pcm24_conversion":convert}), true)["accepted_revision"].clone()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
fn pin(revision: &Value) -> Value {
    json!({"artifact_id":revision["artifact_id"],"revision_id":revision["id"]})
}

#[test]
fn resumed_read_keeps_historical_material_precise_and_rejects_bad_identity_or_bytes() {
    let f = Fixture::new();
    let mut project = ShapeProject::create(&f.path, "Resume").unwrap();
    let source = project.import_raster_bytes(png(10), "Original").unwrap();
    let text = project.create_text_document("Text", "Hello").unwrap();
    let original = project
        .read_accepted(source.artifact_id)
        .unwrap()
        .unwrap()
        .bytes;
    drop(project);
    let source_pin = json!({"artifact_id":source.artifact_id,"revision_id":source.id});
    let initial = f.material(&json!({"action":"read","source":source_pin}), true);
    assert!(initial["material_edit"].is_null());
    let ticket = f.material(
        &json!({"action":"begin","source":source_pin,"name":"Return","editor":"shadow"}),
        true,
    )["ticket"]
        .clone();
    let first = f.accept_return(&ticket, &png(60), "first", false);
    let ticket = f.material(
        &json!({"action":"status","output_artifact_id":first["artifact_id"]}),
        true,
    )["ticket"]
        .clone();
    let second = f.accept_return(&ticket, &png(80), "second", false);
    let read = f.material(&json!({"action":"read","source":pin(&first)}), true);
    assert_eq!(read["revision"], first);
    assert_eq!(read["artifact"]["accepted_revision"], second["id"]);
    assert_eq!(read["source"], pin(&first));
    assert_eq!(read["material_edit"]["source"], source_pin);
    assert_eq!(read["material_edit"]["normalization"], "canonical");
    assert_eq!(
        serde_json::from_value::<ContentDigest>(read["material_edit"]["returned_digest"].clone())
            .unwrap(),
        ContentDigest::from_bytes(&png(60))
    );
    for field in ["content_verified", "accepted", "read_only"] {
        assert_eq!(read[field], true);
    }
    assert_eq!(second["parents"], json!([first["id"]]));
    f.material(&json!({"action":"read","source":{"artifact_id":source.artifact_id,"revision_id":first["id"]}}), false);
    f.material(
        &json!({"action":"read","source":{"artifact_id":text.artifact_id,"revision_id":text.id}}),
        false,
    );
    f.material(&json!({"action":"read","source":{"artifact_id":source.artifact_id,"revision_id":shape_domain::RevisionId::new()}}), false);
    f.material(
        &json!({"action":"read","source":pin(&first),"follow_head":true}),
        false,
    );
    let reopened = ShapeProject::open(&f.path).unwrap();
    assert_eq!(
        reopened
            .read_accepted(source.artifact_id)
            .unwrap()
            .unwrap()
            .bytes,
        original
    );
    assert_eq!(
        reopened
            .read_accepted(serde_json::from_value(second["artifact_id"].clone()).unwrap())
            .unwrap()
            .unwrap()
            .revision
            .id
            .to_string(),
        second["id"]
    );
    drop(reopened);
    let digest: ContentDigest = serde_json::from_value(first["content"]["digest"].clone()).unwrap();
    let digest = digest.to_string();
    let object = f
        .path
        .join("objects/b3")
        .join(&digest[..2])
        .join(&digest[2..]);
    let mut corrupt = fs::read(&object).unwrap();
    corrupt[0] ^= 1;
    fs::write(object, corrupt).unwrap();
    f.material(&json!({"action":"read","source":pin(&first)}), false);
}

#[test]
fn recovered_image_pin_continues_a_bilingual_book_without_changing_its_old_revision() {
    let f = Fixture::new();
    let mut project = ShapeProject::create(&f.path, "Book resume").unwrap();
    let image = project.import_raster_bytes(png(10), "Original").unwrap();
    let en = project
        .create_text_document("English", "[Cover] Moon\n[Title] Night\n[1] Hello")
        .unwrap();
    let zh = project
        .create_text_document("中文", "[封面] 月亮\n[扉页] 夜晚\n[1] 你好")
        .unwrap();
    let image_pin = json!({"artifact_id":image.artifact_id,"revision_id":image.id});
    let composition = json!({"name":"Moon / 月亮","default_language":"zh-CN",
        "languages":{"en":{"artifact_id":en.artifact_id,"revision_id":en.id},"zh-CN":{"artifact_id":zh.artifact_id,"revision_id":zh.id}},
        "pages":[{"kind":"image","source":image_pin,"print_only":false},{"kind":"blank","print_only":false},{"kind":"image","source":image_pin,"print_only":false}],"canvas_width":1024,"canvas_height":768,"font_size":26});
    let candidate = project
        .propose_picture_book(None, serde_json::from_value(composition.clone()).unwrap())
        .unwrap();
    let book = serde_json::to_value(project.accept_picture_book(candidate).unwrap()).unwrap();
    drop(project);
    let ticket = f.material(
        &json!({"action":"begin","source":image_pin,"name":"Moon revision","editor":"shadow"}),
        true,
    )["ticket"]
        .clone();
    let returned = f.accept_return(&ticket, &png(80), "moon-return", false);
    let recovered = f.material(&json!({"action":"read","source":pin(&returned)}), true);
    let previous = f.book(&json!({"action":"read","source":pin(&book)}));
    assert_eq!(previous["composition"]["pages"][2]["source"], image_pin);
    let mut next = previous["composition"].clone();
    next["pages"][2]["source"] = recovered["source"].clone();
    let old = f.book(&json!({"action":"read","source":pin(&book)}));
    assert_eq!(old["composition"], composition);
    let destination = f.root.join("continued.scproj");
    f.book(&json!({"composition":next,"destination":destination}));
    assert!(fs::metadata(destination).unwrap().len() > 0);
    let reopened = ShapeProject::open(&f.path).unwrap();
    assert_eq!(
        reopened
            .read_accepted(image.artifact_id)
            .unwrap()
            .unwrap()
            .revision
            .id,
        image.id
    );
}

fn wave(sample: i32, bits: u16) -> Vec<u8> {
    let width = usize::from(bits / 8);
    let block = bits / 8;
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((36 + u32::from(block) * 2).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16_u32.to_le_bytes());
    bytes.extend(1_u16.to_le_bytes());
    bytes.extend(1_u16.to_le_bytes());
    bytes.extend(24_000_u32.to_le_bytes());
    bytes.extend((24_000_u32 * u32::from(block)).to_le_bytes());
    bytes.extend(block.to_le_bytes());
    bytes.extend(bits.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend((u32::from(block) * 2).to_le_bytes());
    bytes.extend(&sample.to_le_bytes()[..width]);
    bytes.extend(&(-sample).to_le_bytes()[..width]);
    bytes
}

#[test]
fn recovered_normalized_audio_is_usable_as_an_explicit_script_cue_choice() {
    let f = Fixture::new();
    let input = f.root.join("original.wav");
    let original = wave(100, 16);
    fs::write(&input, &original).unwrap();
    let mut project = ShapeProject::create(&f.path, "Audio resume").unwrap();
    let source = project.import_audio_wav(&input, "Original audio").unwrap();
    let source_pin = json!({"artifact_id":source.artifact_id,"revision_id":source.id});
    drop(project);
    let ticket = f.material(
        &json!({"action":"begin","source":source_pin,"name":"Cue edit","editor":"echo"}),
        true,
    )["ticket"]
        .clone();
    let raw = wave(32_768, 24);
    let revision = f.accept_return(&ticket, &raw, "cue-return", true);
    let recovered = f.material(&json!({"action":"read","source":pin(&revision)}), true);
    assert_eq!(
        recovered["material_edit"]["normalization"],
        "pcm24_to_pcm16"
    );
    assert_eq!(recovered["material_edit"]["source"], source_pin);
    assert_eq!(
        serde_json::from_value::<ContentDigest>(
            recovered["material_edit"]["returned_digest"].clone()
        )
        .unwrap(),
        ContentDigest::from_bytes(&raw)
    );
    assert_ne!(
        recovered["revision"]["content"]["digest"],
        recovered["material_edit"]["returned_digest"]
    );
    let mut options = shape_domain::speech_script::SpeechScriptOptions::default();
    options.roles.insert(
        "Narrator".into(),
        shape_domain::PresetVoiceSelection::new(
            shape_domain::PresetVoiceAlias::new("speech.voice.en.warm_male.v1").unwrap(),
            "20260922.1",
        )
        .unwrap(),
    );
    options.cues.insert(
        "turn".into(),
        shape_domain::speech_script::SpeechCueAction::Audio {
            content: serde_json::from_value(recovered["revision"]["content"].clone()).unwrap(),
        },
    );
    let script = f.root.join("script.txt");
    fs::write(
        &script,
        "[role: Narrator]\n[cue: turn]\n[speaker: Narrator]\nHello, 世界.\n[audio: turn]\n",
    )
    .unwrap();
    let bindings = f.root.join("bindings.json");
    let bindings_bytes = serde_json::to_vec(&options).unwrap();
    fs::write(&bindings, &bindings_bytes).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_shape-cli"))
        .arg("script-check")
        .arg(script)
        .arg("--bindings")
        .arg(bindings)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["prerequisites"]["ready"], true);
    assert_eq!(report["prerequisites"]["unresolved_cues"], json!([]));
    assert_eq!(report["synthesis_checked"], false);
    assert_eq!(
        report["bindings_digest"],
        ContentDigest::from_bytes(&bindings_bytes).to_string()
    );
    let reopened = ShapeProject::open(&f.path).unwrap();
    let accepted: ArtifactRevision = serde_json::from_value(revision).unwrap();
    assert_eq!(
        reopened.read_revision_content(accepted.id).unwrap().bytes,
        wave(128, 16)
    );
    assert_eq!(
        reopened
            .read_accepted(source.artifact_id)
            .unwrap()
            .unwrap()
            .bytes,
        original
    );
    let read = ShapeProject::inspect_material(
        &f.path,
        WorkingInput {
            artifact_id: source.artifact_id,
            revision_id: source.id,
        },
    )
    .unwrap();
    assert_eq!(read.revision.id, source.id);
}
