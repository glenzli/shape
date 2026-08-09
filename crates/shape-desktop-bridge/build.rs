//! Generates the narrow CXX ABI consumed by the Qt desktop target.

fn main() {
    println!("cargo:rerun-if-changed=src/lib.rs");
    let mut build = cxx_build::bridge("src/lib.rs");
    if std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc") {
        build.flag("/W4").flag("/permissive-");
    } else {
        build.flag("-Wall").flag("-Wextra").flag("-Wpedantic");
    }
    build.compile("shape-desktop-bridge-cxx");
}
