use std::fs::{self, File};
use std::io::{Read, copy};
use std::path::Path;

use anyhow::{Result, bail};

const CUAD_ZIP_URL: &str = "https://zenodo.org/records/4595826/files/CUAD_v1.zip?download=1";
const ZIP_FILE_PATH: &str = "CUAD_v1.zip";
const EXTRACT_DIR: &str = "data";

pub fn setup() -> Result<()> {
    download_archive(CUAD_ZIP_URL, ZIP_FILE_PATH)?;
    extract_archive(ZIP_FILE_PATH, EXTRACT_DIR)?;
    fs::remove_file(ZIP_FILE_PATH)?;
    Ok(())
}

fn download_archive(url: &str, output_path: &str) -> Result<()> {
    println!("Streaming download from: {url}");
    let mut response = fetch_ok(url)?;
    write_to_file(&mut response, output_path)?;
    println!("Archive successfully downloaded to: {output_path}");
    Ok(())
}

fn fetch_ok(url: &str) -> Result<reqwest::blocking::Response> {
    let response = reqwest::blocking::Client::new()
        .get(url)
        .header(reqwest::header::USER_AGENT, "mask")
        .send()?;
    if response.status().is_success() {
        return Ok(response);
    }
    bail!("Download failed with HTTP status: {}", response.status())
}

fn extract_archive(zip_path: &str, target_dir: &str) -> Result<()> {
    println!("Unpacking archive into '{target_dir}'...");
    let file = File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for index in 0..archive.len() {
        extract_entry(&mut archive, index, target_dir)?;
    }

    println!("Extraction complete: all PDFs, TXTs, and CSV labels are in '{target_dir}'");
    Ok(())
}

fn extract_entry(
    archive: &mut zip::ZipArchive<File>,
    index: usize,
    target_dir: &str,
) -> Result<()> {
    let mut entry = archive.by_index(index)?;
    let Some(enclosed_path) = entry.enclosed_name() else {
        return Ok(());
    };

    let outpath = Path::new(target_dir).join(enclosed_path);
    if entry.is_dir() {
        fs::create_dir_all(&outpath)?;
        return Ok(());
    }

    write_zip_file(&mut entry, &outpath)
}

fn write_zip_file(entry: &mut impl Read, outpath: &Path) -> Result<()> {
    if let Some(parent) = outpath.parent() {
        fs::create_dir_all(parent)?;
    }
    write_to_file(entry, outpath)
}

fn write_to_file(reader: &mut impl Read, path: impl AsRef<Path>) -> Result<()> {
    let mut dest = File::create(path)?;
    copy(reader, &mut dest)?;
    Ok(())
}
