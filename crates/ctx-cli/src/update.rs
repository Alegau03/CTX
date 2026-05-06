use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use clap::{Args, ValueEnum};
use serde::Deserialize;

const INSTALLER_UPDATE_COMMAND: &str =
    "curl -fsSL https://raw.githubusercontent.com/Alegau03/CTX/main/scripts/install.sh | sh";

#[derive(Debug, Clone, Args)]
pub struct UpdateArgs {
    #[arg(long, default_value_t = false)]
    pub check: bool,

    #[arg(long, default_value_t = false)]
    pub yes: bool,

    #[arg(long, value_enum)]
    pub channel: Option<UpdateChannel>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum UpdateChannel {
    Installer,
    Cargo,
    Npm,
    Brew,
}

impl UpdateChannel {
    fn as_str(self) -> &'static str {
        match self {
            Self::Installer => "installer",
            Self::Cargo => "cargo",
            Self::Npm => "npm",
            Self::Brew => "brew",
        }
    }

    fn update_command(self) -> &'static str {
        match self {
            Self::Installer => INSTALLER_UPDATE_COMMAND,
            Self::Cargo => "cargo install ctx-cli --force",
            Self::Npm => "npm update -g @alegau/ctx-bin",
            Self::Brew => "brew upgrade ctx",
        }
    }
}

#[derive(Debug, Deserialize)]
struct InstallerMarker {
    channel: String,
    version: Option<String>,
    install_dir: Option<String>,
    binary_path: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum DetectionSource {
    Explicit,
    Marker,
    PathHeuristic,
    Unknown,
}

impl DetectionSource {
    fn as_str(self) -> &'static str {
        match self {
            Self::Explicit => "explicit",
            Self::Marker => "installer-marker",
            Self::PathHeuristic => "path-heuristic",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug)]
struct Detection {
    channel: Option<UpdateChannel>,
    source: DetectionSource,
}

pub fn run_update(args: &UpdateArgs) -> Result<()> {
    let current_version = env!("CARGO_PKG_VERSION");
    let latest_version = resolve_latest_version()?;
    let detection = detect_channel(args.channel)?;
    let update_available = latest_version != current_version;

    println!("current_version: {current_version}");
    println!("latest_version: {latest_version}");
    println!(
        "channel: {}",
        detection
            .channel
            .map(UpdateChannel::as_str)
            .unwrap_or("unknown")
    );
    println!("detection: {}", detection.source.as_str());
    println!("update_available: {update_available}");

    if !update_available {
        println!("status: up to date");
        return Ok(());
    }

    if args.check {
        println!("status: update available");
        return Ok(());
    }

    match detection.channel {
        Some(channel) => {
            let command = channel.update_command();
            println!("recommended_command: {command}");
            if args.yes && channel == UpdateChannel::Installer {
                execute_installer_update()?;
                println!("update_result: installer command completed");
            } else if args.yes {
                println!("automatic_update: disabled_for_channel");
                println!("run_manually: {command}");
            } else {
                println!("next: {command}");
            }
        }
        None => {
            println!("status: install channel could not be detected safely");
            println!("installer: {}", UpdateChannel::Installer.update_command());
            println!("cargo: {}", UpdateChannel::Cargo.update_command());
            println!("npm: {}", UpdateChannel::Npm.update_command());
            println!("brew: {}", UpdateChannel::Brew.update_command());
        }
    }

    Ok(())
}

fn resolve_latest_version() -> Result<String> {
    if let Ok(version) = env::var("CTX_UPDATE_LATEST_VERSION") {
        let trimmed = version.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    let repo_slug = env::var("CTX_REPO_SLUG").unwrap_or_else(|_| "Alegau03/CTX".to_string());
    let output = Command::new("curl")
        .args([
            "-fsSL",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "User-Agent: ctx-cli",
            &format!("https://api.github.com/repos/{repo_slug}/releases/latest"),
        ])
        .output()
        .context("failed to run curl while resolving the latest CTX release")?;

    if !output.status.success() {
        bail!("failed to resolve the latest CTX release from GitHub");
    }

    let value: serde_json::Value = serde_json::from_slice(&output.stdout)
        .context("failed to parse latest GitHub release response")?;
    let tag = value["tag_name"]
        .as_str()
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .context("latest GitHub release response did not include a tag_name")?;
    Ok(tag.trim_start_matches('v').to_string())
}

fn detect_channel(explicit: Option<UpdateChannel>) -> Result<Detection> {
    if let Some(channel) = explicit {
        return Ok(Detection {
            channel: Some(channel),
            source: DetectionSource::Explicit,
        });
    }

    if let Some(marker) = read_installer_marker()? {
        if marker.channel.trim() == "installer" {
            return Ok(Detection {
                channel: Some(UpdateChannel::Installer),
                source: DetectionSource::Marker,
            });
        }
    }

    if let Some(path) = current_binary_path() {
        let normalized = path.display().to_string().to_lowercase();
        let channel = if normalized.contains("ctx-bin")
            || normalized.contains("node_modules")
            || normalized.contains("npm")
        {
            Some(UpdateChannel::Npm)
        } else if normalized.contains("cellar")
            || normalized.contains("homebrew")
            || normalized.contains("linuxbrew")
        {
            Some(UpdateChannel::Brew)
        } else if normalized.contains(".cargo/bin/ctx")
            || normalized.contains(".cargo\\bin\\ctx.exe")
        {
            Some(UpdateChannel::Cargo)
        } else {
            None
        };

        if channel.is_some() {
            return Ok(Detection {
                channel,
                source: DetectionSource::PathHeuristic,
            });
        }
    }

    Ok(Detection {
        channel: None,
        source: DetectionSource::Unknown,
    })
}

fn current_binary_path() -> Option<PathBuf> {
    if let Ok(path) = env::var("CTX_UPDATE_SELF_PATH") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }

    env::current_exe().ok()
}

fn read_installer_marker() -> Result<Option<InstallerMarker>> {
    let path = installer_marker_path()?;
    if !path.is_file() {
        return Ok(None);
    }

    let body = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read installer marker at {}", path.display()))?;
    let marker: InstallerMarker = serde_json::from_str(&body)
        .with_context(|| format!("failed to parse installer marker at {}", path.display()))?;
    let _ = (
        marker.version.as_deref(),
        marker.install_dir.as_deref(),
        marker.binary_path.as_deref(),
    );
    Ok(Some(marker))
}

fn installer_marker_path() -> Result<PathBuf> {
    if let Ok(path) = env::var("CTX_INSTALL_MARKER_PATH") {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }

    let data_root = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| Path::new(&home).join(".local/share")))
        .context("could not determine a data directory for the CTX installer marker")?;
    Ok(data_root.join("ctx/install.json"))
}

fn execute_installer_update() -> Result<()> {
    let status = Command::new("sh")
        .arg("-c")
        .arg(INSTALLER_UPDATE_COMMAND)
        .status()
        .context("failed to launch the CTX installer update command")?;

    if !status.success() {
        bail!("the CTX installer update command failed");
    }

    Ok(())
}
