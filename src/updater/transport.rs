//! HTTPS to the release service through the system's TLS (Security.framework)
//! and trusted roots, with no redirects: the polled manifest, revalidated with
//! `If-None-Match`, and resumable downloads into a `.partial` file that a later
//! attempt continues with `Range`. Requests carry nothing about the install
//! beyond `User-Agent: spaceman/<channel>/<version>`. A download is checked
//! against the manifest's size and SHA-256 before any byte is used.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use sha2::{Digest, Sha256};
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};

use super::protocol::Target;

const MAX_MANIFEST_BYTES: u64 = 256 * 1024;
const MANIFEST_TIMEOUT: Duration = Duration::from_secs(60);
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(15 * 60);
/// Requests one download makes before the check fails; each resumes where the
/// last one stopped, and the next check resumes again.
const DOWNLOAD_ATTEMPTS: u32 = 3;

/// What every request names itself to the release service.
pub(super) fn user_agent(channel: &str, version: &str) -> String {
    format!("spaceman/{channel}/{version}")
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .tls_config(
            TlsConfig::builder()
                .provider(TlsProvider::NativeTls)
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .https_only(true)
        // A redirect could leave the release service's files; none is followed.
        .max_redirects(0)
        .http_status_as_error(false)
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_global(Some(timeout))
        .build()
        .into()
}

type Response = ureq::http::Response<ureq::Body>;

fn send(request: ureq::http::Request<()>, timeout: Duration) -> Result<Response, String> {
    agent(timeout)
        .run(request)
        .map_err(|error| format!("couldn't reach the update service: {error}"))
}

fn request(url: &str, user_agent: &str) -> ureq::http::request::Builder {
    ureq::http::Request::get(url).header("User-Agent", user_agent)
}

/// The manifest request, revalidating `etag` when one is cached.
fn manifest_request(
    url: &str,
    user_agent: &str,
    etag: Option<&str>,
) -> Result<ureq::http::Request<()>, String> {
    let mut builder = request(url, user_agent);
    if let Some(etag) = etag {
        builder = builder.header("If-None-Match", etag);
    }
    builder.body(()).map_err(|error| error.to_string())
}

/// What the service answered a manifest request.
#[derive(Debug, Eq, PartialEq)]
pub(super) enum Reply {
    NotModified,
    Manifest { etag: Option<String>, body: String },
}

/// The last manifest the service sent, revalidated with its ETag so an
/// unchanged channel costs a `304 Not Modified` and no body.
#[derive(Debug, Default)]
pub(super) struct ManifestCache {
    /// The manifest the ETag and body belong to; switching channels changes it.
    url: Option<String>,
    etag: Option<String>,
    body: Option<String>,
}

impl ManifestCache {
    /// The manifest at `url`, as text: the cached one when the service
    /// answers 304.
    pub(super) fn fetch(&mut self, url: &str, user_agent: &str) -> Result<String, String> {
        self.revalidate(url, |etag| {
            let mut response = send(manifest_request(url, user_agent, etag)?, MANIFEST_TIMEOUT)?;
            match response.status().as_u16() {
                304 => Ok(Reply::NotModified),
                200 => {
                    let etag = response
                        .headers()
                        .get("etag")
                        .and_then(|value| value.to_str().ok())
                        .map(|value| value.trim().to_owned());
                    let body = response
                        .body_mut()
                        .with_config()
                        .limit(MAX_MANIFEST_BYTES)
                        .read_to_string()
                        .map_err(|error| format!("couldn't read the update manifest: {error}"))?;
                    Ok(Reply::Manifest { etag, body })
                }
                status => Err(format!("the update service answered HTTP {status}")),
            }
        })
    }

    /// Asks `send` for `url` with the cached ETag, if any, and keeps what it
    /// answers.
    fn revalidate(
        &mut self,
        url: &str,
        send: impl FnOnce(Option<&str>) -> Result<Reply, String>,
    ) -> Result<String, String> {
        if self.url.as_deref() != Some(url) {
            *self = Self {
                url: Some(url.to_owned()),
                ..Self::default()
            };
        }
        let etag = self.etag.as_deref().filter(|_| self.body.is_some());
        match send(etag)? {
            Reply::NotModified => self
                .body
                .clone()
                .ok_or_else(|| "the update service answered 304 for nothing cached".into()),
            Reply::Manifest { etag, body } => {
                self.etag = etag.filter(|etag| valid_etag(etag));
                self.body = Some(body.clone());
                Ok(body)
            }
        }
    }
}

fn valid_etag(etag: &str) -> bool {
    !etag.is_empty() && etag.len() <= 1024 && !etag.chars().any(char::is_control)
}

/// Downloads `target` from `url` into `directory/<sha256>/<name>` and returns
/// the file once its size and SHA-256 match the manifest. A finished download
/// of the same bytes is reused, and an interrupted one resumes where it
/// stopped: its `.partial` file stays until the bytes are whole.
pub(super) fn download(
    url: &str,
    user_agent: &str,
    target: &Target,
    directory: &Path,
) -> Result<PathBuf, String> {
    let name = url
        .rsplit('/')
        .next()
        .filter(|name| valid_file_name(name))
        .ok_or("the update file name is not usable")?;
    let folder = directory.join(&target.sha256);
    let file = folder.join(name);
    if file.is_file() && verify(&file, target).is_ok() {
        return Ok(file);
    }
    fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
    let partial = folder.join(format!("{name}.partial"));
    let length = |path: &Path| fs::metadata(path).map_or(0, |metadata| metadata.len());
    if length(&partial) > target.size {
        fs::remove_file(&partial).map_err(|error| error.to_string())?;
    }
    let mut failures = 0;
    while length(&partial) < target.size {
        let have = length(&partial);
        let result = fetch_piece(url, user_agent, target.size, &partial, have);
        // Any progress counts: a dropped connection resumes from there.
        if length(&partial) > have {
            failures = 0;
            continue;
        }
        failures += 1;
        if failures >= DOWNLOAD_ATTEMPTS {
            return Err(result
                .err()
                .unwrap_or_else(|| "the update download stopped making progress".into()));
        }
    }
    if let Err(error) = verify(&partial, target) {
        // Whole but wrong: resuming can never make these bytes right.
        drop(fs::remove_file(&partial));
        return Err(error);
    }
    fs::rename(&partial, &file).map_err(|error| error.to_string())?;
    Ok(file)
}

/// The download request: the whole file, or from byte `have` on.
fn download_request(
    url: &str,
    user_agent: &str,
    have: u64,
) -> Result<ureq::http::Request<()>, String> {
    let mut builder = request(url, user_agent);
    if have > 0 {
        builder = builder.header("Range", format!("bytes={have}-"));
    }
    builder.body(()).map_err(|error| error.to_string())
}

fn fetch_piece(
    url: &str,
    user_agent: &str,
    size: u64,
    partial: &Path,
    have: u64,
) -> Result<(), String> {
    let mut response = send(download_request(url, user_agent, have)?, DOWNLOAD_TIMEOUT)?;
    let status = response.status().as_u16();
    let range = response
        .headers()
        .get("content-range")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let body = response.body_mut().with_config().limit(u64::MAX).reader();
    write_piece(status, range.as_deref(), body, partial, have, size)
}

/// Writes one response into `partial`, which holds `have` of `size` bytes:
/// `206 Partial Content` from `have` appends, `200` (no range support) starts
/// over, and `416` drops bytes the service won't continue.
fn write_piece(
    status: u16,
    content_range: Option<&str>,
    mut body: impl Read,
    partial: &Path,
    have: u64,
    size: u64,
) -> Result<(), String> {
    let mut options = OpenOptions::new();
    options.create(true).mode(0o600);
    let start = match status {
        206 if have > 0 => {
            let start = content_range
                .and_then(|range| range.strip_prefix("bytes "))
                .and_then(|range| range.split_once('-'))
                .and_then(|(start, _)| start.trim().parse::<u64>().ok());
            if start != Some(have) {
                return Err(
                    "the update service sent a different part of the file than asked for".into(),
                );
            }
            options.append(true);
            have
        }
        200 => {
            options.write(true).truncate(true);
            0
        }
        416 => {
            drop(fs::remove_file(partial));
            return Err("the update service couldn't resume the download".into());
        }
        status => return Err(format!("the update service answered HTTP {status}")),
    };
    let mut out = options.open(partial).map_err(|error| error.to_string())?;
    let mut written = start;
    let mut buffer = vec![0; 64 * 1024];
    loop {
        let read = body
            .read(&mut buffer)
            .map_err(|error| format!("the update download failed: {error}"))?;
        if read == 0 {
            break;
        }
        if written + read as u64 > size {
            drop(out);
            drop(fs::remove_file(partial));
            return Err("the update is larger than the manifest says".into());
        }
        out.write_all(&buffer[..read])
            .map_err(|error| error.to_string())?;
        written += read as u64;
    }
    out.flush().map_err(|error| error.to_string())
}

fn valid_file_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 200
        && bytes[0].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._+-".contains(byte))
}

/// Checks `path` against the manifest's size and SHA-256.
pub(super) fn verify(path: &Path, target: &Target) -> Result<(), String> {
    let size = fs::metadata(path).map_err(|error| error.to_string())?.len();
    if size != target.size {
        return Err(format!(
            "the update has {size} bytes, expected {}",
            target.size
        ));
    }
    if sha256_hex(path).map_err(|error| error.to_string())? != target.sha256 {
        return Err("the update's SHA-256 does not match the manifest".into());
    }
    Ok(())
}

pub(super) fn sha256_hex(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

/// Removes downloads other than `keep` (a target's SHA-256).
pub(super) fn prune(directory: &Path, keep: Option<&str>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        if keep != name.to_str() && name.len() == 64 && entry.path().is_dir() {
            drop(fs::remove_dir_all(entry.path()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("spaceman-update-{name}-{}", std::process::id()));
        drop(fs::remove_dir_all(&path));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn requests_name_the_app_channel_and_version() {
        assert_eq!(user_agent("dev", "0.0.0-dev-8"), "spaceman/dev/0.0.0-dev-8");
        assert_eq!(user_agent("stable", "0.3.2"), "spaceman/stable/0.3.2");
    }

    #[test]
    fn downloads_are_checked_against_size_and_sha256() {
        let directory = temporary("verify");
        let path = directory.join("file");
        fs::write(&path, b"abc").unwrap();
        let mut target = Target {
            url: "files/x/file".into(),
            size: 3,
            sha256: sha256_hex(&path).unwrap(),
        };
        assert_eq!(
            target.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(verify(&path, &target).is_ok());
        target.size = 4;
        assert!(verify(&path, &target).is_err());
        target.size = 3;
        target.sha256 = "a".repeat(64);
        assert!(verify(&path, &target).is_err());
        drop(fs::remove_dir_all(directory));
    }

    #[test]
    fn a_finished_download_is_reused_without_the_network_and_pruned() {
        let directory = temporary("reuse");
        let source = directory.join("source");
        fs::write(&source, b"bytes").unwrap();
        let sha256 = sha256_hex(&source).unwrap();
        let target = Target {
            url: format!("files/{sha256}/Spaceman.zip"),
            size: 5,
            sha256: sha256.clone(),
        };
        fs::create_dir_all(directory.join(&sha256)).unwrap();
        fs::copy(&source, directory.join(&sha256).join("Spaceman.zip")).unwrap();
        let file = download(
            "https://invalid.invalid/Spaceman.zip",
            "spaceman/dev/0.0.0-dev-1",
            &target,
            &directory,
        )
        .unwrap();
        assert_eq!(fs::read(&file).unwrap(), b"bytes");
        // A whole `.partial` from an interrupted run needs no request either.
        fs::remove_file(&file).unwrap();
        fs::copy(
            &source,
            directory.join(&sha256).join("Spaceman.zip.partial"),
        )
        .unwrap();
        let file = download(
            "https://invalid.invalid/Spaceman.zip",
            "spaceman/dev/0.0.0-dev-1",
            &target,
            &directory,
        )
        .unwrap();
        assert_eq!(fs::read(&file).unwrap(), b"bytes");
        prune(&directory, Some(&sha256));
        assert!(directory.join(&sha256).exists());
        prune(&directory, None);
        assert!(!directory.join(&sha256).exists());
        assert!(!valid_file_name("../x"));
        assert!(!valid_file_name(".hidden"));
        assert!(valid_file_name("Spaceman.zip"));
        drop(fs::remove_dir_all(directory));
    }

    fn header<'a>(request: &'a ureq::http::Request<()>, name: &str) -> Option<&'a str> {
        request
            .headers()
            .get(name)
            .and_then(|value| value.to_str().ok())
    }

    #[test]
    fn manifest_polls_revalidate_the_cached_etag() {
        let url = "https://anoma.ly/release/spaceman/desktop/dev.json";
        let ua = "spaceman/dev/0.0.0-dev-8";
        let first = manifest_request(url, ua, None).unwrap();
        assert_eq!(header(&first, "If-None-Match"), None);
        assert_eq!(header(&first, "User-Agent"), Some(ua));
        let again = manifest_request(url, ua, Some("\"v1\"")).unwrap();
        assert_eq!(header(&again, "If-None-Match"), Some("\"v1\""));

        let mut cache = ManifestCache::default();
        let body = cache
            .revalidate(url, |etag| {
                assert_eq!(etag, None);
                Ok(Reply::Manifest {
                    etag: Some("\"v1\"".into()),
                    body: "{}".into(),
                })
            })
            .unwrap();
        assert_eq!(body, "{}");
        // An unchanged channel: the saved ETag goes out and 304 reuses the body.
        let body = cache
            .revalidate(url, |etag| {
                assert_eq!(etag, Some("\"v1\""));
                Ok(Reply::NotModified)
            })
            .unwrap();
        assert_eq!(body, "{}");
        // A new manifest replaces both; one without an ETag is never revalidated.
        cache
            .revalidate(url, |_| {
                Ok(Reply::Manifest {
                    etag: None,
                    body: "{\"v\":2}".into(),
                })
            })
            .unwrap();
        cache
            .revalidate(url, |etag| {
                assert_eq!(etag, None);
                Ok(Reply::Manifest {
                    etag: Some("\"v3\"".into()),
                    body: "3".into(),
                })
            })
            .unwrap();
        // Another channel's manifest starts over.
        let stable = "https://anoma.ly/release/spaceman/desktop/stable.json";
        let error = cache
            .revalidate(stable, |etag| {
                assert_eq!(etag, None);
                Ok(Reply::NotModified)
            })
            .unwrap_err();
        assert!(error.contains("304"), "{error}");
        assert!(!valid_etag(""));
        assert!(!valid_etag("\"a\nb\""));
    }

    #[test]
    fn resumed_downloads_ask_for_the_rest_and_append_partial_content() {
        let url = "https://anoma.ly/release/spaceman/files/x/Spaceman.zip";
        let ua = "spaceman/dev/0.0.0-dev-8";
        assert_eq!(
            header(&download_request(url, ua, 0).unwrap(), "Range"),
            None
        );
        assert_eq!(
            header(&download_request(url, ua, 4).unwrap(), "Range"),
            Some("bytes=4-")
        );

        let directory = temporary("resume");
        let partial = directory.join("Spaceman.zip.partial");
        fs::write(&partial, b"abcd").unwrap();
        write_piece(206, Some("bytes 4-7/8"), &b"efgh"[..], &partial, 4, 8).unwrap();
        assert_eq!(fs::read(&partial).unwrap(), b"abcdefgh");

        // A range that starts elsewhere is refused and leaves the bytes alone.
        fs::write(&partial, b"abcd").unwrap();
        assert!(write_piece(206, Some("bytes 0-7/8"), &b"abcdefgh"[..], &partial, 4, 8).is_err());
        assert_eq!(fs::read(&partial).unwrap(), b"abcd");

        // No range support: the whole file, from the start.
        write_piece(200, None, &b"ABCDEFGH"[..], &partial, 4, 8).unwrap();
        assert_eq!(fs::read(&partial).unwrap(), b"ABCDEFGH");

        // More than the manifest's size, or a range the service won't serve,
        // drops the partial file.
        fs::write(&partial, b"abcd").unwrap();
        assert!(write_piece(200, None, &b"123456789"[..], &partial, 4, 8).is_err());
        assert!(!partial.exists());
        fs::write(&partial, b"abcd").unwrap();
        assert!(write_piece(416, None, &b""[..], &partial, 4, 8).is_err());
        assert!(!partial.exists());
        drop(fs::remove_dir_all(directory));
    }

    #[test]
    fn an_interrupted_download_keeps_its_partial_file() {
        let directory = temporary("interrupted");
        let target = Target {
            url: "files/x/Spaceman.zip".into(),
            size: 8,
            sha256: "a".repeat(64),
        };
        let folder = directory.join(&target.sha256);
        fs::create_dir_all(&folder).unwrap();
        fs::write(folder.join("Spaceman.zip.partial"), b"abcd").unwrap();
        // Nothing listens on port 1: every attempt fails before a byte arrives.
        let error = download(
            "https://127.0.0.1:1/Spaceman.zip",
            "spaceman/dev/0.0.0-dev-8",
            &target,
            &directory,
        )
        .unwrap_err();
        assert!(error.contains("couldn't reach"), "{error}");
        assert_eq!(
            fs::read(folder.join("Spaceman.zip.partial")).unwrap(),
            b"abcd"
        );
        drop(fs::remove_dir_all(directory));
    }
}
