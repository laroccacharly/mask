use std::fs;
use std::path::Path;

use anyhow::Result;

pub fn extract(input: &Path, output: &Path) -> Result<()> {
    let text = pdf_extract::extract_text(input)?;
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, text)?;
    Ok(())
}
