//! Files on the disk: `<root>/<team>/<file>`, written beside then renamed, so a file is whole or absent.

use std::io::ErrorKind;
use std::path::PathBuf;

use async_trait::async_trait;
use axum::body::Bytes;

use super::{BlobError, Blobs};
use crate::util::{random, Id};

pub struct Disk {
    root: PathBuf,
}

fn err(e: std::io::Error) -> BlobError {
    BlobError(e.kind().to_string())
}

impl Disk {
    pub fn new(root: PathBuf) -> Result<Self, BlobError> {
        std::fs::create_dir_all(&root).map_err(err)?;
        Ok(Disk { root })
    }

    fn path(&self, team: &Id, file: &Id) -> PathBuf {
        self.root.join(hex::encode(team)).join(hex::encode(file))
    }
}

#[async_trait]
impl Blobs for Disk {
    async fn put(&self, team: &Id, file: &Id, bytes: Bytes) -> Result<(), BlobError> {
        let path = self.path(team, file);
        let dir = path.parent().expect("a file has a folder").to_path_buf();
        tokio::fs::create_dir_all(&dir).await.map_err(err)?;
        let tmp = dir.join(format!(".{}.part", hex::encode(random::<8>())));
        tokio::fs::write(&tmp, &bytes).await.map_err(err)?;
        if let Err(e) = tokio::fs::rename(&tmp, &path).await {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(err(e));
        }
        Ok(())
    }

    async fn get(&self, team: &Id, file: &Id) -> Result<Option<Bytes>, BlobError> {
        match tokio::fs::read(self.path(team, file)).await {
            Ok(b) => Ok(Some(Bytes::from(b))),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(None),
            Err(e) => Err(err(e)),
        }
    }

    async fn delete(&self, team: &Id, file: &Id) -> Result<(), BlobError> {
        match tokio::fs::remove_file(self.path(team, file)).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(()),
            Err(e) => Err(err(e)),
        }
    }
}
