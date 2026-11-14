//! Files in an S3-compatible store (MinIO, Garage, AWS), path-style (`<endpoint>/<bucket>/<prefix><key>`), each
//! request signed by AWS Signature Version 4, by hand: three calls (PUT, GET, DELETE) do not need an SDK.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use axum::body::Bytes;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use super::{key, BlobError, Blobs};
use crate::config::S3Config;
use crate::util::Id;

pub struct S3 {
    config: S3Config,
    http: reqwest::Client,
}

fn err(e: impl std::fmt::Display) -> BlobError {
    BlobError(e.to_string())
}

fn hmac(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC takes any key");
    mac.update(data);
    mac.finalize().into_bytes().into()
}

fn sha256_hex(data: &[u8]) -> String {
    hex::encode(Sha256::digest(data))
}

/// `YYYYMMDD'T'HHMMSS'Z'`, from the Unix time.
pub fn amz_date(time: SystemTime) -> String {
    let secs = time
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs() as i64;
    let (days, rem) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // Days to a civil date (H. Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}{month:02}{day:02}T{:02}{:02}{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// Encodes a path as SigV4 wants it: every byte but the unreserved ones and `/`.
fn uri_encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for b in path.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~' | b'/') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// The `Authorization` header of a request without a query string. `headers` are lowercased names and their values,
/// `host` among them.
#[allow(clippy::too_many_arguments)]
pub fn authorization(
    method: &str,
    path: &str,
    headers: &[(&str, String)],
    payload_hash: &str,
    amz_date: &str,
    region: &str,
    access_key: &str,
    secret_key: &str,
) -> String {
    let mut headers: Vec<(&str, &str)> = headers.iter().map(|(k, v)| (*k, v.trim())).collect();
    headers.sort_by(|a, b| a.0.cmp(b.0));
    let canonical_headers: String = headers.iter().map(|(k, v)| format!("{k}:{v}\n")).collect();
    let signed: Vec<&str> = headers.iter().map(|(k, _)| *k).collect();
    let signed = signed.join(";");
    let canonical = format!(
        "{method}\n{}\n\n{canonical_headers}\n{signed}\n{payload_hash}",
        uri_encode_path(path)
    );
    let date = &amz_date[..8];
    let scope = format!("{date}/{region}/s3/aws4_request");
    let to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
        sha256_hex(canonical.as_bytes())
    );
    let k_date = hmac(format!("AWS4{secret_key}").as_bytes(), date.as_bytes());
    let k_region = hmac(&k_date, region.as_bytes());
    let k_service = hmac(&k_region, b"s3");
    let k_signing = hmac(&k_service, b"aws4_request");
    let signature = hex::encode(hmac(&k_signing, to_sign.as_bytes()));
    format!("AWS4-HMAC-SHA256 Credential={access_key}/{scope}, SignedHeaders={signed}, Signature={signature}")
}

impl S3 {
    pub fn new(config: S3Config) -> Result<Self, BlobError> {
        reqwest::Url::parse(&config.endpoint).map_err(err)?;
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(err)?;
        Ok(S3 { config, http })
    }

    async fn send(
        &self,
        method: reqwest::Method,
        team: &Id,
        file: &Id,
        body: Option<Bytes>,
    ) -> Result<reqwest::Response, BlobError> {
        let endpoint = reqwest::Url::parse(&self.config.endpoint).map_err(err)?;
        let base_path = endpoint.path().trim_end_matches('/');
        let path = format!(
            "{base_path}/{}/{}{}",
            self.config.bucket,
            self.config.prefix,
            key(team, file)
        );
        let mut url = endpoint.clone();
        url.set_path(&uri_encode_path(&path));
        let host = match endpoint.port() {
            Some(port) => format!("{}:{port}", endpoint.host_str().unwrap_or_default()),
            None => endpoint.host_str().unwrap_or_default().to_string(),
        };
        let payload_hash = sha256_hex(body.as_deref().unwrap_or_default());
        let date = amz_date(SystemTime::now());
        let auth = authorization(
            method.as_str(),
            &path,
            &[
                ("host", host),
                ("x-amz-content-sha256", payload_hash.clone()),
                ("x-amz-date", date.clone()),
            ],
            &payload_hash,
            &date,
            &self.config.region,
            &self.config.access_key,
            &self.config.secret_key,
        );
        let mut request = self
            .http
            .request(method, url)
            .header("x-amz-content-sha256", payload_hash)
            .header("x-amz-date", date)
            .header("authorization", auth);
        if let Some(body) = body {
            request = request.body(body);
        }
        request.send().await.map_err(err)
    }
}

#[async_trait]
impl Blobs for S3 {
    async fn put(&self, team: &Id, file: &Id, bytes: Bytes) -> Result<(), BlobError> {
        let r = self
            .send(reqwest::Method::PUT, team, file, Some(bytes))
            .await?;
        if !r.status().is_success() {
            return Err(BlobError(format!(
                "S3 PUT answered {}",
                r.status().as_u16()
            )));
        }
        Ok(())
    }

    async fn get(&self, team: &Id, file: &Id) -> Result<Option<Bytes>, BlobError> {
        let r = self.send(reqwest::Method::GET, team, file, None).await?;
        match r.status().as_u16() {
            200 => Ok(Some(r.bytes().await.map_err(err)?)),
            404 => Ok(None),
            s => Err(BlobError(format!("S3 GET answered {s}"))),
        }
    }

    async fn delete(&self, team: &Id, file: &Id) -> Result<(), BlobError> {
        let r = self.send(reqwest::Method::DELETE, team, file, None).await?;
        match r.status().as_u16() {
            200 | 204 | 404 => Ok(()),
            s => Err(BlobError(format!("S3 DELETE answered {s}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AWS's own example: « GET Object », in « Signature Calculations for the Authorization Header: Transferring
    /// Payload in a Single Chunk (AWS Signature Version 4) ».
    #[test]
    fn the_signature_matches_aws_example() {
        let empty = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
        let auth = authorization(
            "GET",
            "/test.txt",
            &[
                ("host", "examplebucket.s3.amazonaws.com".into()),
                ("range", "bytes=0-9".into()),
                ("x-amz-content-sha256", empty.into()),
                ("x-amz-date", "20130524T000000Z".into()),
            ],
            empty,
            "20130524T000000Z",
            "us-east-1",
            "AKIAIOSFODNN7EXAMPLE",
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
        );
        assert_eq!(
            auth,
            "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request, \
             SignedHeaders=host;range;x-amz-content-sha256;x-amz-date, \
             Signature=f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41"
        );
    }

    #[test]
    fn the_date_is_utc() {
        assert_eq!(
            amz_date(UNIX_EPOCH + Duration::from_secs(1_369_353_600)),
            "20130524T000000Z"
        );
        assert_eq!(
            amz_date(UNIX_EPOCH + Duration::from_secs(951_782_400 + 3_723)),
            "20000229T010203Z"
        );
    }
}
