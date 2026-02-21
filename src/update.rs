//! Self-update logic for the `merlin self-update` command.
//!
//! Checks GitHub Releases for a newer version, downloads the correct binary
//! for the current platform, verifies its SHA-256 checksum, and atomically
//! replaces the running executable.
//!
//! # Update flow
//!
//! 1. Query the GitHub Releases API for the latest tag
//! 2. Compare against the running version ([`CURRENT_VERSION`])
//! 3. If up-to-date, print a confirmation and exit
//! 4. Download the platform-appropriate binary to a temp file
//! 5. Download and verify the `.sha256` checksum
//! 6. Replace the current executable atomically via rename
//!
//! # Platform binary names
//!
//! | Platform | Binary |
//! |---|---|
//! | macOS Apple Silicon | `merlin-darwin-arm64` |
//! | macOS Intel | `merlin-darwin-amd64` |
//! | Linux x86-64 | `merlin-linux-amd64-musl` |
//! | Linux ARM64 | `merlin-linux-arm64-musl` |
//! | Windows x86-64 | `merlin-windows-amd64.exe` |

use std::io::Write;

use serde::Deserialize;

use crate::error::{MerlinError, Result};

/// The version string baked into this binary at compile time.
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

const REPO: &str = "Arunachalamkalimuthu/merlin-ai-code-review";
const RELEASES_API: &str = "https://api.github.com/repos/Arunachalamkalimuthu/merlin-ai-code-review/releases/latest";

// ── GitHub API types ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    body: Option<String>,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Check whether a newer release exists without downloading anything.
///
/// Prints the result to stdout.  Returns `Ok(())` in all cases — network or
/// API errors are printed as warnings so `--check` never breaks CI.
pub async fn check_for_update() -> Result<()> {
    match fetch_latest_release().await {
        Ok(release) => {
            let latest = release.tag_name.trim_start_matches('v');
            let current = CURRENT_VERSION;

            if is_newer(latest, current) {
                println!(
                    "A new version of Merlin is available: v{latest} (you have v{current})\n\
                     Run `merlin self-update` to upgrade, or visit:\n  {}",
                    release.html_url
                );
            } else {
                println!("Merlin v{current} is up to date.");
            }
        }
        Err(e) => {
            eprintln!("Could not check for updates: {e}");
        }
    }
    Ok(())
}

/// Download and install the latest release binary.
///
/// If the running version is already the latest, prints a confirmation and
/// returns without downloading.
///
/// # Errors
///
/// Returns an error if:
/// - The GitHub API is unreachable
/// - No binary is available for the current platform
/// - The downloaded checksum does not match
/// - The current executable path cannot be determined
pub async fn self_update(force: bool) -> Result<()> {
    println!("Merlin self-update — current version: v{CURRENT_VERSION}");
    println!("Fetching latest release information…");

    let release = fetch_latest_release().await?;
    let latest = release.tag_name.trim_start_matches('v');

    if !force && !is_newer(latest, CURRENT_VERSION) {
        println!("Already up to date (v{CURRENT_VERSION}). Nothing to do.");
        return Ok(());
    }

    println!("New version available: v{latest}");

    if let Some(ref notes) = release.body {
        let trimmed = notes.trim();
        if !trimmed.is_empty() {
            println!("\n--- Release notes ---");
            // Print at most the first 20 lines to avoid flooding the terminal
            for line in trimmed.lines().take(20) {
                println!("  {line}");
            }
            println!("--- End of notes ---\n");
        }
    }

    // Determine the asset name for this platform
    let asset_name = platform_asset_name()?;
    let tag = &release.tag_name;

    let base_url = format!(
        "https://github.com/{REPO}/releases/download/{tag}"
    );
    let binary_url = format!("{base_url}/{asset_name}");
    let checksum_url = format!("{base_url}/{asset_name}.sha256");

    println!("Downloading {asset_name}…");

    let client = reqwest::Client::builder()
        .user_agent(format!("merlin/{CURRENT_VERSION}"))
        .build()
        .map_err(MerlinError::Http)?;

    // Download binary into a temp file
    let binary_bytes = download_bytes(&client, &binary_url).await?;

    // Download and verify checksum
    println!("Verifying checksum…");
    match download_bytes(&client, &checksum_url).await {
        Ok(checksum_bytes) => {
            let checksum_str = String::from_utf8_lossy(&checksum_bytes);
            let expected = checksum_str.split_whitespace().next().unwrap_or("").to_string();
            let actual = sha256_hex(&binary_bytes);

            if !expected.is_empty() && actual != expected {
                return Err(MerlinError::Other(format!(
                    "Checksum mismatch — download may be corrupt.\n  Expected: {expected}\n  Got:      {actual}"
                )));
            }
            println!("Checksum OK.");
        }
        Err(_) => {
            eprintln!("Warning: could not download checksum file — skipping verification.");
        }
    }

    // Locate the running executable
    let current_exe = std::env::current_exe().map_err(|e| {
        MerlinError::Other(format!("Could not determine current executable path: {e}"))
    })?;

    // Write new binary to a temp file next to the current exe
    let tmp_path = current_exe.with_extension("tmp");
    {
        let mut tmp = std::fs::File::create(&tmp_path).map_err(|e| {
            MerlinError::Other(format!(
                "Could not write to {}: {e}\nTry running with sudo.",
                tmp_path.display()
            ))
        })?;
        tmp.write_all(&binary_bytes)?;
    }

    // Set executable permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&tmp_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&tmp_path, perms)?;
    }

    // On Windows the running exe cannot be overwritten directly.
    // Rename the old one to .old, then move the new one into place.
    #[cfg(windows)]
    {
        let old_path = current_exe.with_extension("old");
        let _ = std::fs::remove_file(&old_path); // ignore error if not present
        std::fs::rename(&current_exe, &old_path).map_err(|e| {
            MerlinError::Other(format!("Could not rename current exe: {e}"))
        })?;
    }

    // Atomic rename: tmp → current exe
    std::fs::rename(&tmp_path, &current_exe).map_err(|e| {
        MerlinError::Other(format!(
            "Could not replace {}: {e}\nTry running with sudo.",
            current_exe.display()
        ))
    })?;

    println!(
        "\nMerlin updated successfully: v{CURRENT_VERSION} → v{latest}\n\
         Run `merlin --version` to confirm."
    );

    Ok(())
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Fetch the latest GitHub release metadata.
async fn fetch_latest_release() -> Result<GithubRelease> {
    let client = reqwest::Client::builder()
        .user_agent(format!("merlin/{CURRENT_VERSION}"))
        .build()
        .map_err(MerlinError::Http)?;

    let response = client
        .get(RELEASES_API)
        .header("Accept", "application/vnd.github.v3+json")
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(MerlinError::Other(format!(
            "GitHub API returned {status}: {body}"
        )));
    }

    let release: GithubRelease = response.json().await?;
    Ok(release)
}

/// Download the full body of a URL into a `Vec<u8>`.
async fn download_bytes(client: &reqwest::Client, url: &str) -> Result<Vec<u8>> {
    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        return Err(MerlinError::Other(format!(
            "Failed to download {url}: HTTP {status}"
        )));
    }

    let bytes = response.bytes().await?;
    Ok(bytes.to_vec())
}

/// Return the release asset filename for the current platform.
fn platform_asset_name() -> Result<String> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let name = match (os, arch) {
        ("macos", "aarch64") => "merlin-darwin-arm64",
        ("macos", "x86_64")  => "merlin-darwin-amd64",
        ("linux", "x86_64")  => "merlin-linux-amd64-musl",
        ("linux", "aarch64") => "merlin-linux-arm64-musl",
        ("windows", "x86_64") => "merlin-windows-amd64.exe",
        _ => {
            return Err(MerlinError::Other(format!(
                "No pre-built binary for {os}/{arch}. Build from source: \
                 cargo install --git https://github.com/{REPO}"
            )));
        }
    };

    Ok(name.to_string())
}

/// Compute the lowercase hex SHA-256 digest of `data`.
fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Return `true` when `latest` is strictly greater than `current`.
///
/// Compares only the numeric major.minor.patch components; pre-release
/// suffixes are ignored.
fn is_newer(latest: &str, current: &str) -> bool {
    parse_semver(latest) > parse_semver(current)
}

/// Parse `"major.minor.patch"` into `(u32, u32, u32)`, ignoring suffixes.
fn parse_semver(v: &str) -> (u32, u32, u32) {
    let mut parts = v.split('.');
    let major = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch = parts
        .next()
        .and_then(|s| s.split('-').next())   // strip pre-release suffix
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    (major, minor, patch)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_newer_detects_patch() {
        assert!(is_newer("0.1.2", "0.1.1"));
        assert!(!is_newer("0.1.1", "0.1.1"));
        assert!(!is_newer("0.1.0", "0.1.1"));
    }

    #[test]
    fn is_newer_detects_minor() {
        assert!(is_newer("0.2.0", "0.1.9"));
        assert!(!is_newer("0.1.9", "0.2.0"));
    }

    #[test]
    fn is_newer_detects_major() {
        assert!(is_newer("1.0.0", "0.9.9"));
    }

    #[test]
    fn is_newer_ignores_prerelease_suffix() {
        // "0.1.2-beta" should still be newer than "0.1.1"
        assert!(is_newer("0.1.2-beta", "0.1.1"));
    }

    #[test]
    fn platform_asset_returns_string() {
        // Should not panic on the CI runner's platform
        let result = platform_asset_name();
        // Either succeeds or returns an unsupported-platform error — both are OK
        match result {
            Ok(name) => assert!(name.starts_with("merlin-")),
            Err(_) => {}
        }
    }

    #[test]
    fn sha256_hex_known_value() {
        // SHA-256 of empty string
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
