use std::fs;

use zed_extension_api::{self as zed, Result};

struct IweExtension {
    cached_binary_path: Option<String>,
}

impl IweExtension {
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
        let asset_name = format!(
            "iwe-{}-{}.tar.gz",
            release.version,
            match (platform, arch) {
                (zed::Os::Linux, zed::Architecture::Aarch64) => "aarch64-unknown-linux-gnu",
                (zed::Os::Linux, zed::Architecture::X8664) => "x86_64-unknown-linux-gnu",
                (zed::Os::Mac, _) => "universal-apple-darwin",
                (zed::Os::Windows, _) => todo!("winows not supported at the moment"),
                _ => todo!("unsupported platform"),
            }
        );

        let asset = release
            .assets
            .iter()
            .find(|asset| asset.name == asset_name)
            .ok_or_else(|| format!("no asset found matching {:?}", asset_name))?;

        let version_dir = format!("iwe-{}", release.version);
        fs::create_dir_all(&version_dir).map_err(|e| format!("create directory failure: {e}"))?;

        let archive_path = format!("{version_dir}/release.tar.gz");
        let binary_path = format!("{version_dir}/release.tar.gz/iwes");

        if !fs::metadata(&archive_path).map_or(false, |stat| stat.is_file()) {
            zed::set_language_server_installation_status(
                language_server_id,
                &zed::LanguageServerInstallationStatus::Downloading,
            );

            zed::download_file(
                &asset.download_url,
                &archive_path,
                zed::DownloadedFileType::GzipTar,
            )
            .map_err(|e| format!("file download failure: {e}"))?;

            zed::make_file_executable(&binary_path)?;

            let entries =
                fs::read_dir(".").map_err(|e| format!("reading directory failure {e}"))?;
            for entry in entries {
                let entry =
                    entry.map_err(|e| format!("directory entry etntry read failure {e}"))?;
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
