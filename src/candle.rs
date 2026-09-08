use std::fs;
use std::path::PathBuf;

use anyhow::Result;
use candle_pipelines::text_generation::{Message, Qwen3, TextGenerationPipelineBuilder};

const HF_HOME: &str = "data/huggingface";

pub fn complete(prompt: &str) -> Result<String> {
    configure_hf_home()?;
    let pipeline = TextGenerationPipelineBuilder::qwen3(Qwen3::Size0_6B)
        .temperature(0.0)
        .max_len(128)
        .build()?;
    let messages = vec![Message::user(&format!("{prompt} /no_think"))];
    let output = pipeline.run(&messages)?;
    Ok(output.text)
}

fn configure_hf_home() -> Result<()> {
    fs::create_dir_all(HF_HOME)?;
    let hf_home = PathBuf::from(HF_HOME);
    let hf_home = if hf_home.is_absolute() {
        hf_home
    } else {
        std::env::current_dir()?.join(hf_home)
    };
    unsafe { std::env::set_var("HF_HOME", hf_home) };
    Ok(())
}
