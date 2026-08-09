//! Durable BLAKE3 content-addressed object storage.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use shape_domain::{ContentDigest, ContentRef};
use uuid::Uuid;

use crate::StoreError;

#[derive(Debug, Clone)]
pub(super) struct ObjectStore {
    objects_root: PathBuf,
}

impl ObjectStore {
    pub(super) fn create(bundle_root: &Path) -> Result<Self, StoreError> {
        let objects_root = bundle_root.join("objects").join("b3");
        fs::create_dir_all(&objects_root)?;
        Ok(Self { objects_root })
    }

    pub(super) fn open(bundle_root: &Path) -> Result<Self, StoreError> {
        let objects_root = bundle_root.join("objects").join("b3");
        if !objects_root.is_dir() {
            return Err(StoreError::BundleMissing(objects_root));
        }
        Ok(Self { objects_root })
    }

    pub(super) fn publish(
        &self,
        bytes: &[u8],
        media_type: impl Into<String>,
    ) -> Result<ContentRef, StoreError> {
        let digest = ContentDigest::from_bytes(bytes);
        let content = ContentRef::new(
            digest,
            media_type,
            u64::try_from(bytes.len()).map_err(|_| StoreError::IntegerOutOfRange)?,
        )?;
        let destination = self.path_for(digest);
        if destination.exists() {
            Self::verify_path(&destination, digest, bytes.len())?;
            return Ok(content);
        }

        let parent = destination
            .parent()
            .expect("content paths always have a digest prefix directory");
        fs::create_dir_all(parent)?;
        let temporary = parent.join(format!(".shape-object-{}.tmp", Uuid::now_v7()));
        let publish_result = (|| {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            match fs::rename(&temporary, &destination) {
                Ok(()) => Ok(()),
                Err(_error) if destination.exists() => {
                    let _ = fs::remove_file(&temporary);
                    Self::verify_path(&destination, digest, bytes.len())?;
                    Ok(())
                }
                Err(error) => Err(StoreError::Io(error)),
            }
        })();
        if publish_result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        publish_result?;

        Ok(content)
    }

    pub(super) fn read(&self, content: &ContentRef) -> Result<Vec<u8>, StoreError> {
        let path = self.path_for(content.digest);
        let bytes = fs::read(&path)?;
        let expected_length =
            usize::try_from(content.byte_length).map_err(|_| StoreError::IntegerOutOfRange)?;
        if bytes.len() != expected_length || ContentDigest::from_bytes(&bytes) != content.digest {
            return Err(StoreError::CorruptObject { path });
        }
        Ok(bytes)
    }

    fn verify_path(
        path: &Path,
        expected_digest: ContentDigest,
        expected_length: usize,
    ) -> Result<(), StoreError> {
        let bytes = fs::read(path)?;
        if bytes.len() != expected_length || ContentDigest::from_bytes(&bytes) != expected_digest {
            return Err(StoreError::CorruptObject {
                path: path.to_owned(),
            });
        }
        Ok(())
    }

    fn path_for(&self, digest: ContentDigest) -> PathBuf {
        let hex = digest.to_string();
        self.objects_root.join(&hex[..2]).join(&hex[2..])
    }
}

#[cfg(test)]
mod tests;
