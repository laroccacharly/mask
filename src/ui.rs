use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

const BUILD_HASH_NAME: &str = ".build-hash";
const SKIP_DIRS: &[&str] = &["node_modules", "dist", "dist-ssr"];

pub struct Ui {
    dir: PathBuf,
}

impl Ui {
    pub fn new(dir: PathBuf) -> Result<Self> {
        let ui = Self { dir };
        let dist_dir = ui.dist_dir();
        fs::create_dir_all(dist_dir)?;
        Ok(ui)
    }

    pub fn from_workspace() -> Result<Self> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let ui_dir = manifest_dir.join("ui");
        Self::new(ui_dir)
    }

    pub fn dist_dir(&self) -> PathBuf {
        self.dir.join("dist")
    }

    pub fn assets_exist(&self) -> bool {
        self.dist_dir().join("index.html").is_file()
    }

    pub fn build(&self) -> Result<()> {
        if !self.dir.is_dir() {
            bail!("ui directory not found: {}", self.dir.display());
        }

        let source_hash = self.source_hash()?;
        if self.assets_exist() && self.stored_build_hash()?.as_deref() == Some(source_hash.as_str())
        {
            println!("ui build up to date, skipping");
            return Ok(());
        }

        if !self.dir.join("node_modules").is_dir() {
            let status = Command::new("bun")
                .arg("install")
                .current_dir(&self.dir)
                .status()
                .context("failed to run bun install")?;
            if !status.success() {
                bail!("ui install failed with exit code {status}");
            }
        }

        let status = Command::new("bun")
            .args(["run", "build"])
            .current_dir(&self.dir)
            .status()
            .context("failed to run bun")?;
        if !status.success() {
            bail!("ui build failed with exit code {status}");
        }

        let build_hash_path = self.build_hash_path();
        fs::write(build_hash_path, source_hash)?;
        Ok(())
    }

    fn build_hash_path(&self) -> PathBuf {
        self.dist_dir().join(BUILD_HASH_NAME)
    }

    fn stored_build_hash(&self) -> Result<Option<String>> {
        let path = self.build_hash_path();
        if !path.is_file() {
            return Ok(None);
        }
        Ok(Some(fs::read_to_string(path)?.trim().to_string()))
    }

    fn source_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        collect_source_files(&self.dir, &mut files)?;
        files.sort();
        Ok(files)
    }

    fn source_hash(&self) -> Result<String> {
        let mut digest = Sha256::new();
        for path in self.source_files()? {
            let relative = path
                .strip_prefix(&self.dir)?
                .to_string_lossy()
                .replace('\\', "/");
            let relative_bytes = relative.as_bytes();
            digest.update(relative_bytes);
            digest.update(b"\0");
            let contents = fs::read(&path)?;
            digest.update(contents);
            digest.update(b"\0");
        }
        Ok(format!("{:x}", digest.finalize()))
    }
}

fn collect_source_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            let name = name.as_ref();
            if SKIP_DIRS.contains(&name) {
                continue;
            }
            collect_source_files(&path, out)?;
            continue;
        }
        if let Some("ts" | "tsx") = path.extension().and_then(|ext| ext.to_str()) {
            out.push(path);
        }
    }
    Ok(())
}
