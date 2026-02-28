use std::fs;

use zed_extension_api::{self as zed, Result};

struct IweExtension {
    cached_binary_path: Option<String>,
}

struct PlatformInfo {
    target_triple: &'static str,
    archive_ext: &'static str,
    binary_name: &'static str,
    download_type: zed::DownloadedFileType,
}

impl IweExtension {
    fn platform_info(platform: zed::Os, arch: zed::Architecture) -> Result<PlatformInfo> {
        match (platform, arch) {
            (zed::Os::Mac, _) => Ok(PlatformInfo {
                target_triple: "universal-apple-darwin",
                archive_ext: "tar.gz",
                binary_name: "iwes",
                download_type: zed::DownloadedFileType::GzipTar,
            }),
            (zed::Os::Linux, zed::Architecture::Aarch64) => Ok(PlatformInfo {
                target_triple: "aarch64-unknown-linux-gnu",
                archive_ext: "tar.gz",
                binary_name: "iwes",
                download_type: zed::DownloadedFileType::GzipTar,
            }),
            (zed::Os::Linux, zed::Architecture::X8664) => Ok(PlatformInfo {
                target_triple: "x86_64-unknown-linux-gnu",
                archive_ext: "tar.gz",
                binary_name: "iwes",
                download_type: zed::DownloadedFileType::GzipTar,
            }),
            (zed::Os::Windows, zed::Architecture::X8664) => Ok(PlatformInfo {
                target_triple: "x86_64-pc-windows-msvc",
                archive_ext: "zip",
                binary_name: "iwes.exe",
                download_type: zed::DownloadedFileType::Zip,
            }),
            (zed::Os::Windows, arch) => {
                Err(format!("Windows {:?} is not yet supported", arch))
            }
            (platform, arch) => {
                Err(format!("Unsupported platform: {:?} {:?}", platform, arch))
            }
        }
    }

    fn language_server_binary_path(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<String> {
        if let Some(path) = &self.cached_binary_path {
            if fs::metadata(path).map_or(false, |stat| stat.is_file()) {
                return Ok(path.clone());
            }
        }

        if let Some(path) = worktree.which("iwes") {
            self.cached_binary_path = Some(path.clone());
            return Ok(path);
        }

        zed::set_language_server_installation_status(
            language_server_id,
            &zed::LanguageServerInstallationStatus::CheckingForUpdate,
        );
        let release = zed::latest_github_release(
            "iwe-org/iwe",
            zed::GithubReleaseOptions {
                require_assets: true,
                pre_release: false,
            },
        )?;

        let (platform, arch) = zed::current_platform();
        let info = Self::platform_info(platform, arch)?;

        let asset_name = format!(
            "{}-{}.{}",
            release.version,
            info.target_triple,
            info.archive_ext,
        );

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no asset found matching {:?}", asset_name))?;

        let version_dir = release.version.clone();
        fs::create_dir_all(&version_dir).map_err(|e| format!("create directory failure: {e}"))?;

        let binary_path = format!("{version_dir}/{}", info.binary_name);

        if !fs::metadata(&binary_path).map_or(false, |stat| stat.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            zed::download_file(&asset.download_url, &version_dir, info.download_type)
                .map_err(|e| format!("file download failure: {e}"))?;

            zed::make_file_executable(&binary_path)?;

            let entries =
                fs::read_dir(".").map_err(|e| format!("reading directory failure {e}"))?;
            for entry in entries {
                let entry =
                    entry.map_err(|e| format!("directory entry read failure {e}"))?;
                if entry.file_name().to_str() != Some(&version_dir) {
                    fs::remove_dir_all(entry.path()).ok();
                }
            }
        }

        self.cached_binary_path = Some(binary_path.clone());
        Ok(binary_path)
    }
}

impl zed::Extension for IweExtension {
    fn new() -> Self {
        Self {
            cached_binary_path: None,
        }
    }

    fn language_server_command(
        &mut self,
        language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> Result<zed::Command> {
        Ok(zed::Command {
            command: self.language_server_binary_path(language_server_id, worktree)?,
            args: vec!["".to_string()],
            env: Default::default(),
        })
    }
}

zed::register_extension!(IweExtension);
