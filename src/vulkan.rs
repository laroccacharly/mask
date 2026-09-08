//! llama.cpp CPU-versus-Vulkan benchmark and setup notes.
//!
//! This repository enables the `llama-cpp-2/vulkan` Cargo feature on Linux and
//! sets `VULKAN_SDK=data/vulkan/usr` in `.cargo/config.toml`. Runtime drivers
//! remain system packages; only development headers are kept in the ignored
//! `data` directory.
//!
//! Arch/Omarchy setup:
//!
//! ```text
//! sudo pacman -S --needed vulkan-radeon vulkan-tools glslang shaderc
//! mkdir -p data/vulkan
//! curl -L "$(pacman -Sp --print-format '%l' vulkan-headers)" -o data/vulkan-headers.pkg.tar.zst
//! curl -L "$(pacman -Sp --print-format '%l' spirv-headers)" -o data/spirv-headers.pkg.tar.zst
//! bsdtar -xf data/vulkan-headers.pkg.tar.zst -C data/vulkan
//! bsdtar -xf data/spirv-headers.pkg.tar.zst -C data/vulkan
//! vulkaninfo --summary
//! cargo run --release -- vulkan --tokens 256
//! ```
//!
//! `vulkaninfo` should list the AMD GPU through RADV. If it only lists a CPU
//! device such as llvmpipe, Vulkan is software-rendered and the benchmark is not
//! measuring the Radeon GPU. `VK_ICD_FILENAMES` can select a specific ICD when
//! several drivers are installed; RADV is normally
//! `/usr/share/vulkan/icd.d/radeon_icd.json`.
//!
//! On Debian or Ubuntu, install `libvulkan-dev`, `mesa-vulkan-drivers`,
//! `vulkan-tools`, `glslang-tools`, `glslc`, and `spirv-headers`. When using
//! system development headers instead of the local SDK, remove the `VULKAN_SDK`
//! entry from `.cargo/config.toml` or point it at the matching SDK root.
//!
//! The first Vulkan build can take several minutes because llama.cpp compiles
//! its shader set. Use a release build for timing, close GPU-heavy applications,
//! use at least 128 generated tokens, and repeat the run after warm-up. Short
//! prompt timings are noisy; generation tokens per second is the more useful
//! comparison. The CPU run uses zero GPU layers and the Vulkan run requests all
//! model layers, while keeping the model, prompt, sampler, and context identical.
//!
//! Common failures:
//!
//! - `Vulkan_INCLUDE_DIR` missing: populate `data/vulkan` or install headers.
//! - `SPIRV-HeadersConfig.cmake` missing: install or extract `spirv-headers`.
//! - No Vulkan GPU reported: verify `amdgpu`, the Mesa RADV ICD, and `/dev/dri`.
//! - Out of device memory: use a smaller GGUF model or offload fewer layers.

use std::num::NonZeroU32;
use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::{LlamaBackendDeviceType, list_llama_ggml_backend_devices};

use crate::llamacpp;

const CONTEXT_SIZE: u32 = 2048;
const BATCH_SIZE: usize = 512;

struct Measurement {
    mode: &'static str,
    load: Duration,
    prompt_tokens: usize,
    prompt: Duration,
    generated_tokens: usize,
    generation: Duration,
}

impl Measurement {
    fn prompt_tokens_per_second(&self) -> f64 {
        self.prompt_tokens as f64 / self.prompt.as_secs_f64()
    }

    fn generation_tokens_per_second(&self) -> f64 {
        self.generated_tokens as f64 / self.generation.as_secs_f64()
    }
}

pub fn benchmark(prompt: &str, max_tokens: usize) -> Result<()> {
    if max_tokens == 0 {
        bail!("tokens must be greater than zero");
    }
    let backend = llamacpp::backend()?;
    let devices = list_llama_ggml_backend_devices();
    for device in &devices {
        println!(
            "device: {} ({}, {:?}, {:.1} GiB free)",
            device.description,
            device.backend,
            device.device_type,
            device.memory_free as f64 / 1024_f64.powi(3)
        );
    }
    if !backend.supports_gpu_offload()
        || !devices.iter().any(|device| {
            matches!(
                device.device_type,
                LlamaBackendDeviceType::Gpu | LlamaBackendDeviceType::IntegratedGpu
            )
        })
    {
        bail!("llama.cpp did not find a Vulkan GPU capable of offloading model layers");
    }

    let options = llamacpp::LlmOptions::default();
    let model_path = llamacpp::download_model(&options)?;
    println!("model: {}", model_path.display());
    let cpu = run(backend, &model_path, prompt, max_tokens, 0, "CPU")?;
    let vulkan = run(backend, &model_path, prompt, max_tokens, 1000, "Vulkan")?;
    print_measurement(&cpu);
    print_measurement(&vulkan);
    println!(
        "Vulkan speedup: prompt {:.2}x, generation {:.2}x",
        vulkan.prompt_tokens_per_second() / cpu.prompt_tokens_per_second(),
        vulkan.generation_tokens_per_second() / cpu.generation_tokens_per_second()
    );
    Ok(())
}

fn run(
    backend: &llama_cpp_2::llama_backend::LlamaBackend,
    model_path: &std::path::Path,
    prompt: &str,
    max_tokens: usize,
    gpu_layers: u32,
    mode: &'static str,
) -> Result<Measurement> {
    let load_started = Instant::now();
    let params = LlamaModelParams::default().with_n_gpu_layers(gpu_layers);
    let model = LlamaModel::load_from_file(backend, model_path, &params)?;
    let load = load_started.elapsed();
    let n_ctx = NonZeroU32::new(CONTEXT_SIZE);
    let context_params = LlamaContextParams::default()
        .with_n_ctx(n_ctx)
        .with_n_batch(BATCH_SIZE as u32);
    let mut context = model.new_context(backend, context_params)?;
    let tokens = model.str_to_token(prompt, AddBos::Always)?;
    if tokens.is_empty() {
        bail!("prompt produced no tokens");
    }

    let mut batch = LlamaBatch::new(BATCH_SIZE, 1);
    let token_count = tokens.len();
    let last_index = i32::try_from(token_count)? - 1;
    let prompt_tokens = tokens.iter().copied();
    for (index, token) in (0_i32..).zip(prompt_tokens) {
        batch.add(token, index, &[0], index == last_index)?;
    }
    let prompt_started = Instant::now();
    context.decode(&mut batch)?;
    let prompt_time = prompt_started.elapsed();

    let mut sampler = LlamaSampler::chain_simple([LlamaSampler::greedy()]);
    let generation_started = Instant::now();
    let mut generated_tokens = 0;
    for position in (last_index + 1..).take(max_tokens) {
        let token = sampler.sample(&context, batch.n_tokens() - 1);
        sampler.accept(token);
        if model.is_eog_token(token) {
            break;
        }
        generated_tokens += 1;
        batch.clear();
        batch.add(token, position, &[0], true)?;
        context.decode(&mut batch)?;
    }
    let generation = generation_started.elapsed();
    Ok(Measurement {
        mode,
        load,
        prompt_tokens: tokens.len(),
        prompt: prompt_time,
        generated_tokens,
        generation,
    })
}

fn print_measurement(measurement: &Measurement) {
    println!(
        "{}: load {:.2}s, prompt {:.2} tok/s ({} tokens), generation {:.2} tok/s ({} tokens)",
        measurement.mode,
        measurement.load.as_secs_f64(),
        measurement.prompt_tokens_per_second(),
        measurement.prompt_tokens,
        measurement.generation_tokens_per_second(),
        measurement.generated_tokens
    );
}
