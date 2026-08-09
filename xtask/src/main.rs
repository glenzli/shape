//! Small checked-in repository task runner.

use std::{env, error::Error, io, process::Command};

fn main() {
    if let Err(error) = run() {
        eprintln!("xtask: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    match env::args().nth(1).as_deref() {
        Some("check") => check(),
        Some("format") => command("cargo", &["fmt", "--all"]),
        Some("doctor") => doctor(),
        Some("desktop") => {
            command("cmake", &["--preset", "desktop-dev"])?;
            command("cmake", &["--build", "--preset", "desktop-dev"])
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: cargo xtask <check|format|doctor|desktop>",
        )
        .into()),
    }
}

fn check() -> Result<(), Box<dyn Error>> {
    command("cargo", &["fmt", "--all", "--", "--check"])?;
    command(
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    command("cargo", &["test", "--workspace"])?;
    command("cmake", &["--preset", "native-dev"])?;
    command("cmake", &["--build", "--preset", "native-dev"])
}

fn doctor() -> Result<(), Box<dyn Error>> {
    command("rustc", &["--version"])?;
    command("cargo", &["--version"])?;
    command("cmake", &["--version"])?;
    command("ninja", &["--version"])
}

fn command(program: &str, arguments: &[&str]) -> Result<(), Box<dyn Error>> {
    let status = Command::new(program).args(arguments).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "{program} {} failed with {status}",
            arguments.join(" ")
        ))
        .into())
    }
}
