//! Black-box contract for the public, read-only script diagnostics command.

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{Value, json};
use shape_domain::ContentDigest;

struct Fixture(PathBuf);

impl Fixture {
    fn new(bytes: &[u8]) -> Self {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("shape-script-check-{}-{stamp}", std::process::id()));
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("script.txt"), bytes).unwrap();
        Self(directory)
    }

    fn path(&self) -> PathBuf {
        self.0.join("script.txt")
    }

    fn run(&self) -> Output {
        run(&self.path())
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn run(path: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_shape-cli"))
        .arg("script-check")
        .arg(path)
        .output()
        .unwrap()
}

fn report(output: &Output, code: i32) -> Value {
    assert_eq!(
        output.status.code(),
        Some(code),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema"], "shape.cli.script-check@20261002.1");
    assert_eq!(value["synthesis_checked"], false);
    value
}

#[test]
fn valid_bilingual_script_reports_exact_identity_and_authored_notes() {
    let source = "[role: Narrator]\n[cue: turn; sound: beep]\n[speaker: Narrator]\n[语言：中文]\n[note: Check Shape pronunciation.]\nHello, 世界.\n[repeat: 2; gap: 1.25s]\nAgain.\n[pause: 0.5s]\n[end-repeat]\n[audio: turn]\n";
    let fixture = Fixture::new(source.as_bytes());
    let value = report(&fixture.run(), 0);
    assert_eq!(
        value["grammar_revision"],
        shape_domain::speech_script::SPEECH_SCRIPT_REVISION
    );
    assert_eq!(value["valid"], true);
    assert_eq!(
        value["source_digest"],
        ContentDigest::from_bytes(source.as_bytes()).to_string()
    );
    assert_eq!(value["source_bytes"], source.len());
    assert_eq!(value["roles"], json!(["Narrator"]));
    assert_eq!(value["cues"], json!(["turn"]));
    assert_eq!(value["note_lines"], json!([5]));
    assert_eq!(value["explicit_pause_ms"], 2250);
    assert_eq!(value["issues"], json!([]));
    assert_eq!(fs::read(fixture.path()).unwrap(), source.as_bytes());
    assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1);
}

#[test]
fn invalid_syntax_is_machine_readable_and_does_not_echo_source() {
    let fixture =
        Fixture::new(b"Hello.\n[pause: private-invalid-value]\n[repeat: 2; gap: 1s]\nAgain.");
    let output = fixture.run();
    let value = report(&output, 2);
    assert_eq!(value["valid"], false);
    assert_eq!(
        value["issues"],
        json!([
            {"line": 2, "code": "invalid_pause"},
            {"line": 3, "code": "unclosed_repeat"},
        ])
    );
    assert!(
        !String::from_utf8(output.stdout)
            .unwrap()
            .contains("private-invalid-value")
    );
}

#[test]
fn exact_file_limit_accepts_64_kib_and_rejects_one_more_byte() {
    let mut bytes = vec![b' '; 65_536];
    bytes[..6].copy_from_slice(b"Hello.");
    let fixture = Fixture::new(&bytes);
    assert_eq!(report(&fixture.run(), 0)["source_bytes"], 65_536);
    bytes.push(b' ');
    fs::write(fixture.path(), &bytes).unwrap();
    let output = fixture.run();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr).unwrap().contains("65536"));
    assert_eq!(fs::read(fixture.path()).unwrap(), bytes);
}

#[test]
fn invalid_utf8_and_non_file_inputs_fail_without_a_report() {
    let fixture = Fixture::new(&[0xff, 0xfe]);
    assert!(
        String::from_utf8(fixture.run().stderr)
            .unwrap()
            .contains("UTF-8")
    );
    assert!(
        String::from_utf8(run(&fixture.0).stderr)
            .unwrap()
            .contains("regular file")
    );
    for path in [
        fixture.path(),
        fixture.0.clone(),
        fixture.0.join("missing.txt"),
    ] {
        let output = run(&path);
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn usage_rejects_missing_and_extra_arguments() {
    let fixture = Fixture::new(b"Hello.");
    for arguments in [
        vec!["script-check".into()],
        vec![
            "script-check".into(),
            fixture.path().into_os_string(),
            "extra".into(),
        ],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_shape-cli"))
            .args(arguments)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .contains("script-check")
        );
    }
}
