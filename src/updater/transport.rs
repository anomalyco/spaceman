//! HTTPS to the release service through the system's TLS (Security.framework)
//! and trusted roots, with no redirects. Requests carry nothing about the
//! install beyond `User-Agent: spaceman/<channel>/<version>`. A download is
//! checked against the manifest's size and SHA-256 before any byte is used.

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

fn get(
    url: &str,
    user_agent: &str,
    timeout: Duration,
) -> Result<ureq::http::Response<ureq::Body>, String> {
    let response = agent(timeout)
        .get(url)
        .header("User-Agent", user_agent)
        .call()
        .map_err(|error| format!("couldn't reach the update service: {error}"))?;
    match response.status().as_u16() {
        200 => Ok(response),
        status => Err(format!("the update service answered HTTP {status}")),
    }
}

/// The manifest at `url`, as text.
pub(super) fn fetch_manifest(url: &str, user_agent: &str) -> Result<String, String> {
    let mut response = get(url, user_agent, MANIFEST_TIMEOUT)?;
    response
        .body_mut()
        .with_config()
        .limit(MAX_MANIFEST_BYTES)
        .read_to_string()
        .map_err(|error| format!("couldn't read the update manifest: {error}"))
}

/// Downloads `target` from `url` into `directory/<sha256>/<name>` and returns
/// the file once its size and SHA-256 match the manifest. A finished download
/// of the same bytes is reused.
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
    let result =
        fetch_into(url, user_agent, target, &partial).and_then(|()| verify(&partial, target));
    if let Err(error) = result {
        drop(fs::remove_file(&partial));
        return Err(error);
    }
    fs::rename(&partial, &file).map_err(|error| error.to_string())?;
    Ok(file)
}

fn fetch_into(url: &str, user_agent: &str, target: &Target, path: &Path) -> Result<(), String> {
    let mut response = get(url, user_agent, DOWNLOAD_TIMEOUT)?;
    let mut out = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .map_err(|error| error.to_string())?;
    // One byte past the expected size is enough to prove a larger body.
    let mut body = response
        .body_mut()
        .with_config()
        .limit(target.size.saturating_add(1))
        .reader();
    std::io::copy(&mut body, &mut out)
        .map_err(|error| format!("the update download failed: {error}"))?;
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
        prune(&directory, Some(&sha256));
        assert!(directory.join(&sha256).exists());
        prune(&directory, None);
        assert!(!directory.join(&sha256).exists());
        assert!(!valid_file_name("../x"));
        assert!(!valid_file_name(".hidden"));
        assert!(valid_file_name("Spaceman.zip"));
        drop(fs::remove_dir_all(directory));
    }
}
