use std::{fs, path::PathBuf};

use image::{ColorType, ImageEncoder, codecs::png::PngEncoder};
use shape_domain::ArtifactContentContract;
use uuid::Uuid;

use super::*;

fn test_root() -> PathBuf {
    std::env::temp_dir().join(format!("shape-raster-core-{}", Uuid::now_v7()))
}

fn fixture(path: &Path) {
    let pixels: Vec<u8> = (0_u8..24).flat_map(|value| [value, 20, 40, 255]).collect();
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(&pixels, 6, 4, ColorType::Rgba8.into())
        .unwrap();
    fs::write(path, bytes).unwrap();
}

#[test]
fn import_materializes_an_accepted_canonical_raster_origin() {
    let root = test_root();
    let source = root.with_extension("png");
    fixture(&source);
    let mut project = ShapeProject::create(&root, "Raster Project").unwrap();
    let imported = project.import_raster(&source, "Cover").unwrap();
    let Some(ArtifactContentContract::ImageRaster(import_contract)) =
        imported.content_contract.as_ref()
    else {
        panic!("import contract missing");
    };
    assert_eq!((import_contract.width, import_contract.height), (6, 4));
    assert_eq!(
        project.snapshot().unwrap().artifacts[0].accepted_revision,
        Some(imported.id)
    );
    fs::remove_file(source).unwrap();
    fs::remove_dir_all(root).unwrap();
}
