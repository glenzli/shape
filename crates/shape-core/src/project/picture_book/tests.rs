use super::*;
use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};
use shape_domain::IntentSpec;

struct Fixture {
    root: PathBuf,
    project: ShapeProject,
    request: PictureBookRequest,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("shape-book-{}", uuid::Uuid::now_v7()));
        fs::create_dir(&root).unwrap();
        let mut project = ShapeProject::create(root.join("input.shape"), "Book fixture").unwrap();
        let mut png = Vec::new();
        PngEncoder::new(&mut png)
            .write_image(&[32, 80, 120, 255].repeat(8), 4, 2, ColorType::Rgba8.into())
            .unwrap();
        let image = project.import_raster_bytes(png, "Art").unwrap();
        let en = project
            .create_text_document(
                "English",
                "[Cover] Moon\n[Title] Night\n[Author] Ada\n[1] Hello",
            )
            .unwrap();
        let zh = project
            .create_text_document("中文", "[封面] 月亮\n[扉页] 夜晚\n[作者] 艾达\n[1] 你好")
            .unwrap();
        let pin = |r: &shape_domain::ArtifactRevision| WorkingInput {
            artifact_id: r.artifact_id,
            revision_id: r.id,
        };
        let request = PictureBookRequest {
            name: "Moon / 月亮".into(),
            default_language: "zh-CN".into(),
            languages: BTreeMap::from([("en".into(), pin(&en)), ("zh-CN".into(), pin(&zh))]),
            pages: vec![
                PictureBookPage::Image {
                    source: pin(&image),
                    print_only: false,
                },
                PictureBookPage::Blank { print_only: false },
                PictureBookPage::Image {
                    source: pin(&image),
                    print_only: false,
                },
                PictureBookPage::Blank { print_only: true },
            ],
            canvas_width: 1024,
            canvas_height: 768,
        };
        Self {
            root,
            project,
            request,
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn native_archive_preserves_page_roles_languages_source_bytes_and_independent_repeated_artwork() {
    let f = Fixture::new();
    let snapshot = f.project.snapshot().unwrap();
    let path = f.root.join("book.scproj");
    let receipt = f.project.export_picture_book(&f.request, &path).unwrap();
    assert_eq!(
        receipt.output_blake3,
        shape_domain::ContentDigest::from_bytes(&fs::read(&path).unwrap()).to_string()
    );
    let mut archive = zip::ZipArchive::new(File::open(&path).unwrap()).unwrap();
    let mut read_json = |name: &str| -> Value {
        let mut text = String::new();
        archive
            .by_name(name)
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        serde_json::from_str(&text).unwrap()
    };
    let project = read_json("project.json");
    let origin = read_json("shape-origin.json");
    assert_eq!(project["schema_version"], "20260907.01");
    assert_eq!(project["visible_images"].as_array().unwrap().len(), 4);
    assert_ne!(project["visible_images"][0], project["visible_images"][2]);
    assert_eq!(
        project["page_settings"]["blank://shape-3"]["print_only"],
        true
    );
    assert_eq!(origin["parsed_scripts"]["en"]["page_text"]["2"], "Hello");
    assert_eq!(origin["parsed_scripts"]["zh-CN"]["page_text"]["2"], "你好");
    assert_eq!(origin["parsed_scripts"]["en"]["author"], "Ada");
    for index in [0, 2] {
        let mut bytes = Vec::new();
        archive
            .by_name(project["visible_images"][index].as_str().unwrap())
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        let PictureBookPage::Image { source, .. } = f.request.pages[index] else {
            panic!("image")
        };
        assert_eq!(
            bytes,
            f.project
                .read_revision_content(source.revision_id)
                .unwrap()
                .bytes
        );
    }
    assert_eq!(f.project.snapshot().unwrap(), snapshot);
    assert!(f.project.export_picture_book(&f.request, &path).is_err());
    if let Some(root) = std::env::var_os("SHAPE_BOOK_FIXTURE_DIR") {
        let root = PathBuf::from(root);
        fs::create_dir(&root).unwrap();
        fs::copy(path, root.join("book.scproj")).unwrap();
        fs::write(
            root.join("export.json"),
            serde_json::to_vec_pretty(&receipt).unwrap(),
        )
        .unwrap();
        fs::write(
            root.join("project.json"),
            serde_json::to_vec_pretty(&project).unwrap(),
        )
        .unwrap();
        fs::write(
            root.join("origin.json"),
            serde_json::to_vec_pretty(&origin).unwrap(),
        )
        .unwrap();
    }
}

#[test]
fn changed_input_wrong_media_and_ambiguous_language_roles_publish_nothing() {
    let mut f = Fixture::new();
    let path = f.root.join("book.scproj");
    let original = f.request.clone();
    let body = f
        .project
        .create_text_document("No title", "[Cover] Moon [1] Hello")
        .unwrap();
    f.request.languages.insert(
        "en".into(),
        WorkingInput {
            artifact_id: body.artifact_id,
            revision_id: body.id,
        },
    );
    assert!(f.project.export_picture_book(&f.request, &path).is_err());
    assert!(!path.exists());
    f.request = original.clone();
    f.request.pages[0] = PictureBookPage::Image {
        source: f.request.languages["en"],
        print_only: false,
    };
    assert!(f.project.export_picture_book(&f.request, &path).is_err());
    f.request = original;
    let input = f.request.languages["en"];
    let candidate = f
        .project
        .propose_text(
            input.artifact_id,
            Some(input.revision_id),
            "[Cover] New [Title] Title [1] New body",
            IntentSpec::new("Revise").unwrap(),
            vec![],
        )
        .unwrap();
    f.project.accept_text(candidate).unwrap();
    assert!(f.project.export_picture_book(&f.request, &path).is_err());
    assert!(!path.exists());
    assert!(
        !fs::read_dir(&f.root).unwrap().any(|p| p
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains("partial"))
    );
}

#[test]
fn bounds_internal_writes_and_existing_files_are_rejected_without_clobber() {
    let mut f = Fixture::new();
    let path = f.root.join("existing.scproj");
    fs::write(&path, b"keep").unwrap();
    assert!(f.project.export_picture_book(&f.request, &path).is_err());
    assert_eq!(fs::read(path).unwrap(), b"keep");
    assert!(
        f.project
            .export_picture_book(&f.request, &f.root.join("input.shape/book.scproj"))
            .is_err()
    );
    let path = f.root.join("book.scproj");
    f.request
        .pages
        .resize(65, PictureBookPage::Blank { print_only: false });
    assert!(f.project.export_picture_book(&f.request, &path).is_err());
    assert!(!path.exists());
}
