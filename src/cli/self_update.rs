//! Self-update command - check for and install updates
//!
//! This module provides the `skillsync self update` command which:
//! 1. Queries GitHub Releases API for the latest version
//! 2. Compares with current version
//! 3. Downloads the appropriate binary for the current platform
//! 4. Verifies SHA256 checksum
//! 5. Replaces the current executable

use std::env;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use console::style;
use semver::Version;
use sha2::{Digest, Sha256};

use crate::t;
use crate::i18n::Msg;

/// Platform information determined at compile time
pub struct Platform {
    pub target: &'static str,
    pub binary_name: &'static str,
}

/// Get the current platform
pub const CURRENT_PLATFORM: Platform = {
    #[cfg(all(target_arch = "aarch64", target_os = "macos"))]
    {
        Platform {
            target: "aarch64-apple-darwin",
            binary_name: "skillsync-aarch64-apple-darwin",
        }
    }
    #[cfg(all(target_arch = "x86_64", target_os = "macos"))]
    {
        Platform {
            target: "x86_64-apple-darwin",
            binary_name: "skillsync-x86_64-apple-darwin",
        }
    }
    #[cfg(all(target_arch = "x86_64", target_os = "linux"))]
    {
        Platform {
            target: "x86_64-unknown-linux-gnu",
            binary_name: "skillsync-x86_64-unknown-linux-gnu",
        }
    }
    #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
    {
        Platform {
            target: "aarch64-unknown-linux-gnu",
            binary_name: "skillsync-aarch64-unknown-linux-gnu",
        }
    }
};

/// GitHub repository information
const GITHUB_REPO: &str = "yuzai-code/SkillSync";
const GITHUB_API_URL: &str = "https://api.github.com";

/// Run the self-update command
pub fn run() -> Result<()> {
    println!("{}", t!(Msg::SelfUpdateChecking));

    // 1. Get current version
    let current_version = env!("CARGO_PKG_VERSION");
    println!("{}", t!(Msg::SelfUpdateCurrentVersion {
        version: format!("v{}", current_version)
    }));

    // 2. Query GitHub Releases API
    let release = fetch_latest_release()?;

    // Parse latest version
    let latest_tag = release.tag_name.trim_start_matches('v');
    println!("{}", t!(Msg::SelfUpdateLatestVersion {
        version: release.tag_name.clone()
    }));

    // 3. Compare versions
    let current = Version::parse(current_version)
        .with_context(|| format!("Invalid current version: {}", current_version))?;
    let latest = Version::parse(latest_tag)
        .with_context(|| format!("Invalid latest version: {}", latest_tag))?;

    if current >= latest {
        println!();
        println!(
            "{}",
            style(t!(Msg::SelfUpdateAlreadyUpToDate {
                version: release.tag_name
            }))
            .green()
            .bold()
        );
        return Ok(());
    }

    // 4. Find the binary for current platform
    let binary_name = CURRENT_PLATFORM.binary_name;
    let asset = release
        .assets
        .iter()
        .find(|a| a.name == binary_name)
        .with_context(|| {
            t!(Msg::SelfUpdateNoBinaryForPlatform {
                platform: CURRENT_PLATFORM.target.to_string()
            })
        })?;

    // 5. Find checksum file
    let checksum_asset = release
        .assets
        .iter()
        .find(|a| a.name == "checksums-sha256.txt")
        .context("Checksum file not found in release")?;

    // 6. Download binary
    println!();
    println!("{}", t!(Msg::SelfUpdateDownloading { binary: binary_name.to_string() }));
    let binary_data = download_file(&asset.browser_download_url)?;

    // 7. Download and parse checksum
    let checksum_data = download_file(&checksum_asset.browser_download_url)?;
    let checksum_str = String::from_utf8(checksum_data)
        .context("Checksum file is not valid UTF-8")?;
    let expected_checksum = parse_checksum(&checksum_str, binary_name)
        .with_context(|| format!("Checksum for {} not found", binary_name))?;

    // 8. Verify checksum
    println!("{}", t!(Msg::SelfUpdateVerifying));
    let actual_checksum = compute_sha256(&binary_data);
    if actual_checksum != expected_checksum {
        bail!("{}", t!(Msg::SelfUpdateChecksumMismatch));
    }

    // 9. Replace executable
    let current_exe = env::current_exe()
        .context("Failed to get current executable path")?;

    replace_executable(&current_exe, &binary_data)?;

    println!();
    println!(
        "{}",
        style(t!(Msg::SelfUpdateUpdated {
            version: release.tag_name
        }))
        .green()
        .bold()
    );

    Ok(())
}

/// GitHub Release response
#[derive(Debug, serde::Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<GitHubAsset>,
}

#[derive(Debug, serde::Deserialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
}

/// Fetch the latest release from GitHub
fn fetch_latest_release() -> Result<GitHubRelease> {
    let url = format!("{}/repos/{}/releases/latest", GITHUB_API_URL, GITHUB_REPO);

    let response = ureq::get(&url)
        .set("User-Agent", "SkillSync-Self-Update")
        .set("Accept", "application/vnd.github.v3+json")
        .call()
        .map_err(|e| {
            let err_msg = e.to_string();
            if err_msg.contains("403") || err_msg.contains("rate limit") {
                return anyhow::anyhow!("{}", t!(Msg::SelfUpdateRateLimited));
            }
            anyhow::anyhow!("{}", t!(Msg::SelfUpdateNetworkError { error: err_msg }))
        })?;

    let release: GitHubRelease = serde_json::from_reader(response.into_reader())
        .context("Failed to parse GitHub API response")?;

    Ok(release)
}

/// Download a file from URL
fn download_file(url: &str) -> Result<Vec<u8>> {
    let response = ureq::get(url)
        .set("User-Agent", "SkillSync-Self-Update")
        .call()
        .map_err(|e| {
            anyhow::anyhow!("{}", t!(Msg::SelfUpdateNetworkError { error: e.to_string() }))
        })?;

    let mut data = Vec::new();
    response.into_reader().read_to_end(&mut data)
        .context("Failed to read download data")?;

    Ok(data)
}

/// Parse checksum from checksums file
fn parse_checksum(checksum_data: &str, binary_name: &str) -> Option<String> {
    for line in checksum_data.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() == 2 && parts[1] == binary_name {
            return Some(parts[0].to_string());
        }
    }
    None
}

/// Compute SHA256 hash
fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Replace the current executable with the new one
fn replace_executable(exe_path: &PathBuf, new_data: &[u8]) -> Result<()> {
    // Write to a temporary file first
    let temp_dir = env::temp_dir();
    let temp_file = temp_dir.join("skillsync-update");

    {
        let mut file = File::create(&temp_file)
            .context("Failed to create temporary file")?;
        file.write_all(new_data)
            .context("Failed to write to temporary file")?;
    }

    // Set executable permission on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temp_file, std::fs::Permissions::from_mode(0o755))
            .context("Failed to set executable permissions")?;
    }

    // Try to replace the executable
    if let Err(e) = std::fs::rename(&temp_file, exe_path) {
        // Permission denied - likely need sudo
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            // Clean up temp file
            let _ = std::fs::remove_file(&temp_file);

            println!();
            println!(
                "{}",
                style(t!(Msg::SelfUpdatePermissionDenied)).red()
            );
            println!("{}", t!(Msg::SelfUpdateSudoHint));
            bail!("Permission denied");
        }
        return Err(e).context("Failed to replace executable");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_has_binary_name() {
        assert!(!CURRENT_PLATFORM.binary_name.is_empty());
        assert!(CURRENT_PLATFORM.binary_name.starts_with("skillsync-"));
    }

    #[test]
    fn test_compute_sha256() {
        let data = b"hello world";
        let hash = compute_sha256(data);
        // Known SHA256 of "hello world"
        assert_eq!(
            hash,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_parse_checksum() {
        let checksum_data = "abc123  skillsync-aarch64-apple-darwin\ndef456  skillsync-x86_64-apple-darwin\n";
        let result = parse_checksum(checksum_data, "skillsync-aarch64-apple-darwin");
        assert_eq!(result, Some("abc123".to_string()));

        let result = parse_checksum(checksum_data, "nonexistent");
        assert_eq!(result, None);
    }
}