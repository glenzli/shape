//! Bounded regular-file reads and atomic no-clobber publication for Deck handoffs.
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use super::{DeckHandoffError, invalid};

pub(super) fn read(path: &Path, maximum: u64) -> Result<Vec<u8>, DeckHandoffError> {
    if !path.is_absolute() || !fs::symlink_metadata(path)?.file_type().is_file() {
        return Err(invalid("handoff input must be an absolute regular file"));
    }
    let file = File::open(path)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() > maximum {
        return Err(invalid("handoff file exceeds its byte limit"));
    }
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > maximum {
        return Err(invalid("handoff file exceeds its byte limit"));
    }
    Ok(bytes)
}

pub(super) fn new_path(path: &Path) -> Result<PathBuf, DeckHandoffError> {
    if !path.is_absolute() || fs::symlink_metadata(path).is_ok() {
        return Err(invalid("handoff destination must be a new absolute path"));
    }
    let parent = path
        .parent()
        .ok_or_else(|| invalid("handoff destination needs a parent"))?
        .canonicalize()?;
    Ok(parent.join(
        path.file_name()
            .ok_or_else(|| invalid("handoff destination needs a name"))?,
    ))
}

pub(crate) fn write_new(path: &Path, bytes: &[u8]) -> Result<(), std::io::Error> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("destination needs a parent"))?;
    let temporary =
        Temporary(parent.join(format!(".deck-handoff-{}.partial", uuid::Uuid::now_v7())));
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&temporary.0)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    drop(file);
    fs::hard_link(&temporary.0, path)?;
    Ok(())
}

struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

pub(super) fn output_digest(
    path: &Path,
    expected: u64,
    maximum: u64,
) -> Result<(String, [u8; 8]), DeckHandoffError> {
    if !path.is_absolute()
        || !fs::symlink_metadata(path)?.file_type().is_file()
        || expected == 0
        || expected > maximum
    {
        return Err(invalid(
            "returned output must be a bounded absolute regular file",
        ));
    }
    let mut file = File::open(path)?;
    if !file.metadata()?.is_file() || file.metadata()?.len() != expected {
        return Err(invalid("returned output byte length changed"));
    }
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    let mut prefix = [0_u8; 8];
    let mut length = 0_u64;
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        if length == 0 {
            let size = count.min(8);
            prefix[..size].copy_from_slice(&buffer[..size]);
        }
        length += count as u64;
        if length > expected {
            return Err(invalid("returned output grew during verification"));
        }
        hasher.update(&buffer[..count]);
    }
    if length != expected {
        return Err(invalid("returned output changed during verification"));
    }
    Ok((hasher.finalize().to_hex().to_string(), prefix))
}
