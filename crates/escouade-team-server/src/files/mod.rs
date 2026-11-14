//! The encrypted files (documents, images): their bytes, on the disk or in an S3-compatible store, behind `Blobs`.
//! Their size, author and hour are in the base (`db::Store::reserve_file`), which holds the quota.

pub mod disk;
pub mod s3;

use async_trait::async_trait;
use axum::body::Bytes;

use crate::config::FilesConfig;
use crate::util::Id;

#[derive(Debug, thiserror::Error)]
#[error("the file store: {0}")]
pub struct BlobError(pub String);

#[async_trait]
pub trait Blobs: Send + Sync {
    async fn put(&self, team: &Id, file: &Id, bytes: Bytes) -> Result<(), BlobError>;
    async fn get(&self, team: &Id, file: &Id) -> Result<Option<Bytes>, BlobError>;
    async fn delete(&self, team: &Id, file: &Id) -> Result<(), BlobError>;
}

/// The key of a file: its team's folder, then its id, both in hex.
pub fn key(team: &Id, file: &Id) -> String {
    format!("{}/{}", hex::encode(team), hex::encode(file))
}

pub fn open(config: &FilesConfig) -> Result<Box<dyn Blobs>, BlobError> {
    Ok(match config {
        FilesConfig::Disk(root) => Box::new(disk::Disk::new(root.clone())?),
        FilesConfig::S3(s3) => Box::new(s3::S3::new(s3.clone())?),
    })
}
