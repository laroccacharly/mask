use std::cmp::Reverse;
use std::fs;
use std::io::copy;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Result, anyhow, bail};
use gaze::{PiiClass, Session};
use gliner25_rs::{BoundaryConfig, Precision, SchemaTask, hub};

use crate::gaze::pii_class;
use crate::reject;
use crate::token_formatter::TokenFormatter;

pub use gliner25_rs::BoundaryEngine;
pub use gliner25_rs::boundary::Mention;

const MODEL_DIR: &str = "data/gliner2.5";
const HF_HOME: &str = "data/huggingface";
const ORT_DIR: &str = "data/onnxruntime";
const ORT_VERSION: &str = "1.23.2";

pub fn load_engine() -> Result<BoundaryEngine> {
    fs::create_dir_all(MODEL_DIR)?;
    fs::create_dir_all(HF_HOME)?;
    let hf_home = std::env::current_dir()?.join(HF_HOME);
    unsafe { std::env::set_var("HF_HOME", hf_home) };
    let dylib = ensure_onnxruntime()?;
    ort::init_from(&dylib)?.with_name("mask").commit();
    let config = BoundaryConfig::new(MODEL_DIR)
        .with_precision(Precision::Fp16IoBinding)
        .or_download(hub::GLINER25_MULTI_V1);
    BoundaryEngine::new(config)
}

pub fn extract_entities(
    engine: &mut BoundaryEngine,
    text: &str,
    labels: &[&str],
) -> Result<Vec<Mention>> {
    let tasks = [SchemaTask::Entities(
        labels.iter().map(|label| (*label).to_string()).collect(),
    )];
    Ok(engine.extract_long(text, &tasks)?.mentions)
}

pub fn tokenize_with_extra(
    engine: &mut BoundaryEngine,
    session: &Session,
    text: &str,
    labels: &[&str],
    extra: Vec<(usize, usize, String, String)>,
) -> Result<String> {
    let mut spans: Vec<(usize, usize, PiiClass, String)> = if labels.is_empty() {
        Vec::new()
    } else {
        extract_entities(engine, text, labels)?
            .into_iter()
            .filter_map(|mention| {
                if mention.char_end > text.len() || mention.char_start >= mention.char_end {
                    return None;
                }
                if reject::is_rejected(&mention.text) {
                    return None;
                }
                Some((
                    mention.char_start,
                    mention.char_end,
                    pii_class(&mention.field),
                    mention.text,
                ))
            })
            .collect()
    };
    let extra_spans = extra.into_iter().filter_map(|(start, end, kind, raw)| {
        if !text.is_char_boundary(start)
            || !text.is_char_boundary(end)
            || end > text.len()
            || start >= end
        {
            return None;
        }
        Some((start, end, pii_class(&kind), raw))
    });
    spans.extend(extra_spans);
    if spans.is_empty() {
        return Ok(text.to_string());
    }
    spans.sort_by(|left, right| {
        let starts = left.0.cmp(&right.0);
        let ends = right.1.cmp(&left.1);
        starts.then(ends)
    });

    let mut kept = Vec::new();
    let mut cursor = 0;
    for span in spans {
        if span.0 < cursor {
            continue;
        }
        cursor = span.1;
        kept.push(span);
    }
    kept.sort_by_key(|span| Reverse(span.0));

    let mut encoded = text.to_string();
    for (start, end, class, raw) in kept {
        let formatted = TokenFormatter::casefold(&raw);
        let token = session.tokenize(&class, &formatted)?;
        encoded.replace_range(start..end, &token);
    }
    Ok(encoded)
}

fn ensure_onnxruntime() -> Result<PathBuf> {
    let ort_dir = Path::new(ORT_DIR);
    if let Some(dylib) = find_dylib(ort_dir) {
        return Ok(dylib);
    }

    let archive_stem = ort_archive_stem()?;
    let url = format!(
        "https://github.com/microsoft/onnxruntime/releases/download/v{ORT_VERSION}/{archive_stem}.tgz"
    );
    let archive_path = PathBuf::from("data").join(format!("{archive_stem}.tgz"));
    fs::create_dir_all("data")?;
    download_file(&url, &archive_path)?;
    extract_tgz(&archive_path, ORT_DIR)?;
    fs::remove_file(archive_path)?;

    let ort_dir = Path::new(ORT_DIR);
    find_dylib(ort_dir).ok_or_else(|| anyhow!("ONNX Runtime dylib missing under {ORT_DIR}"))
}

fn find_dylib(root: &Path) -> Option<PathBuf> {
    let name = dylib_name();
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            if path.file_name().is_some_and(|file_name| file_name == name) {
                return Some(path);
            }
        }
    }
    None
}

fn download_file(url: &str, path: &Path) -> Result<()> {
    let mut response = reqwest::blocking::Client::new()
        .get(url)
        .header(reqwest::header::USER_AGENT, "mask")
        .send()?;
    if !response.status().is_success() {
        bail!("ONNX Runtime download failed: {}", response.status());
    }
    let mut dest = fs::File::create(path)?;
    copy(&mut response, &mut dest)?;
    Ok(())
}

fn extract_tgz(archive: &Path, dest: &str) -> Result<()> {
    fs::create_dir_all(dest)?;
    let status = Command::new("tar")
        .args(["-xzf", &archive.to_string_lossy(), "-C", dest])
        .status()?;
    if status.success() {
        return Ok(());
    }
    bail!("tar extract failed with status {status}")
}

fn dylib_name() -> &'static str {
    if cfg!(target_os = "windows") {
        return "onnxruntime.dll";
    }
    if cfg!(target_os = "macos") {
        return "libonnxruntime.dylib";
    }
    "libonnxruntime.so"
}

fn ort_archive_stem() -> Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok("onnxruntime-osx-arm64-1.23.2"),
        ("macos", "x86_64") => Ok("onnxruntime-osx-x86_64-1.23.2"),
        ("linux", "x86_64") => Ok("onnxruntime-linux-x64-1.23.2"),
        ("linux", "aarch64") => Ok("onnxruntime-linux-aarch64-1.23.2"),
        (os, arch) => bail!("no ONNX Runtime build for {os}/{arch}"),
    }
}
