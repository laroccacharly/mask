use std::fmt;
use std::fs;
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow, bail};
use hf_hub::api::sync::ApiBuilder;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaChatMessage, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::token::LlamaToken;
use llama_cpp_2::{LogOptions, send_logs_to_tracing};

const HF_HOME: &str = "data/huggingface";
const CTX_OVERHEAD: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct LlmOptions {
    pub max_new_tokens: i32,
    pub max_think_tokens: usize,
    pub ctx_size: u32,
    pub min_params_b: f32,
}

impl Default for LlmOptions {
    fn default() -> Self {
        Self {
            max_new_tokens: 512,
            max_think_tokens: 128,
            ctx_size: 2048,
            min_params_b: 0.0,
        }
    }
}

struct GgufModel {
    repo: &'static str,
    file: &'static str,
    params_b: f32,
    estimated_ram: u64,
}

const MODELS: &[GgufModel] = &[
    GgufModel {
        repo: "bartowski/Qwen_Qwen3.5-0.8B-GGUF",
        file: "Qwen_Qwen3.5-0.8B-Q4_K_M.gguf",
        params_b: 0.8,
        estimated_ram: 579_615_840 + CTX_OVERHEAD,
    },
    GgufModel {
        repo: "bartowski/Qwen_Qwen3.5-2B-GGUF",
        file: "Qwen_Qwen3.5-2B-Q4_K_M.gguf",
        params_b: 2.0,
        estimated_ram: 1_396_198_496 + CTX_OVERHEAD,
    },
    GgufModel {
        repo: "bartowski/Qwen_Qwen3.5-4B-GGUF",
        file: "Qwen_Qwen3.5-4B-Q4_K_M.gguf",
        params_b: 4.0,
        estimated_ram: 3_013_027_808 + CTX_OVERHEAD,
    },
    GgufModel {
        repo: "bartowski/Qwen_Qwen3.5-9B-GGUF",
        file: "Qwen_Qwen3.5-9B-Q4_K_M.gguf",
        params_b: 9.0,
        estimated_ram: 6_169_341_984 + CTX_OVERHEAD,
    },
];

#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub thinking: String,
    pub content: String,
    pub prompt_tokens: usize,
    pub thinking_tokens: usize,
    pub generated_tokens: usize,
    pub time_to_first_token: Duration,
    pub duration: Duration,
}

impl LlmResponse {
    pub fn tokens_per_second(&self) -> f64 {
        if self.duration.is_zero() {
            return 0.0;
        }
        self.generated_tokens as f64 / self.duration.as_secs_f64()
    }
}

impl fmt::Display for LlmResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "prompt_tokens={} thinking_tokens={} generated_tokens={} time_to_first_token={:?} tokens_per_second={:.1}",
            self.prompt_tokens,
            self.thinking_tokens,
            self.generated_tokens,
            self.time_to_first_token,
            self.tokens_per_second()
        )?;
        writeln!(
            f,
            "thinking:\n{}",
            if self.thinking.is_empty() {
                "(empty)"
            } else {
                &self.thinking
            }
        )?;
        write!(f, "content:\n{}", self.content)
    }
}

pub fn complete(prompt: &str) -> Result<String> {
    let options = LlmOptions::default();
    Ok(generate(prompt, None, &options)?.content)
}

pub(crate) fn complete_json(
    prompt: &str,
    schema_json: &str,
    options: &LlmOptions,
) -> Result<LlmResponse> {
    let grammar = llama_cpp_2::json_schema_to_grammar(schema_json)?;
    generate(prompt, Some(&grammar), options)
}

fn generate(prompt: &str, grammar: Option<&str>, options: &LlmOptions) -> Result<LlmResponse> {
    let backend = backend()?;
    let model_path = download_model(options)?;
    let model_params = LlamaModelParams::default().with_n_gpu_layers(1000);
    let model = LlamaModel::load_from_file(backend, model_path, &model_params)?;
    let context_size = NonZeroU32::new(options.ctx_size)
        .ok_or_else(|| anyhow!("ctx_size must be greater than zero"))?;
    let ctx_params = LlamaContextParams::default().with_n_ctx(Some(context_size));
    let mut ctx = model.new_context(backend, ctx_params)?;

    let template = model.chat_template(None)?;
    let role = "user".to_string();
    let content = if grammar.is_some() {
        prompt.to_string()
    } else {
        format!("{prompt} /no_think")
    };
    let chat = [LlamaChatMessage::new(role, content)?];
    let formatted = model.apply_chat_template(&template, &chat, true)?;
    let formatted = if grammar.is_some() {
        open_thinking(&formatted)
    } else {
        formatted
    };
    let tokens = model.str_to_token(&formatted, AddBos::Never)?;
    if tokens.is_empty() {
        bail!("prompt produced no tokens");
    }

    let prompt_tokens = tokens.len();
    let mut batch = LlamaBatch::new(512, 1);
    let last_index = i32::try_from(prompt_tokens)? - 1;
    for (i, token) in (0_i32..).zip(tokens) {
        if batch.n_tokens() as usize == 512 {
            ctx.decode(&mut batch)?;
            batch.clear();
        }
        batch.add(token, i, &[0], i == last_index)?;
    }
    let started = Instant::now();
    ctx.decode(&mut batch)?;

    let mut decoder = encoding_rs::UTF_8.new_decoder();
    let mut thinking = String::new();
    let mut content = String::new();
    let max_new_tokens = usize::try_from(options.max_new_tokens)?;
    let max_think_tokens = options.max_think_tokens.min(max_new_tokens);
    let mut n_cur = last_index + 1;
    let mut produced = 0usize;
    let mut time_to_first_token = None;
    let mut decode_started = None;
    let mut thinking_tokens = 0usize;
    let mut in_thinking = grammar.is_some();
    let greedy_sampler = LlamaSampler::greedy();
    let mut thinking_sampler = LlamaSampler::chain_simple([greedy_sampler]);
    let mut content_samplers = Vec::new();
    if let Some(grammar) = grammar {
        let grammar_sampler = LlamaSampler::grammar(&model, grammar, "root")?;
        content_samplers.push(grammar_sampler);
    }
    let greedy_sampler = LlamaSampler::greedy();
    content_samplers.push(greedy_sampler);
    let mut content_sampler = LlamaSampler::chain_simple(content_samplers);

    while produced < max_new_tokens {
        if in_thinking && thinking_tokens >= max_think_tokens {
            if !thinking.contains("</think>") {
                let tokens = model.str_to_token("</think>\n", AddBos::Never)?;
                for token in tokens {
                    let piece = piece_from(&model, &mut decoder, token)?;
                    thinking.push_str(&piece);
                    thinking_tokens += 1;
                    push_piece(
                        &mut produced,
                        started,
                        &mut time_to_first_token,
                        &mut decode_started,
                    );
                    batch.clear();
                    batch.add(token, n_cur, &[0], true)?;
                    ctx.decode(&mut batch)?;
                    n_cur += 1;
                }
            }
            in_thinking = false;
            continue;
        }

        let constrained = !in_thinking && grammar.is_some();
        let sampler = if in_thinking {
            &mut thinking_sampler
        } else {
            &mut content_sampler
        };
        let token = next_token(sampler, &ctx, &batch, constrained)?;
        sampler.accept(token);
        if model.is_eog_token(token) {
            break;
        }
        let piece = piece_from(&model, &mut decoder, token)?;
        if in_thinking {
            thinking.push_str(&piece);
            thinking_tokens += 1;
            if thinking.contains("</think>") {
                in_thinking = false;
            }
        } else {
            content.push_str(&piece);
        }
        push_piece(
            &mut produced,
            started,
            &mut time_to_first_token,
            &mut decode_started,
        );
        batch.clear();
        batch.add(token, n_cur, &[0], true)?;
        ctx.decode(&mut batch)?;
        n_cur += 1;
    }

    let duration = decode_started
        .map(|started| started.elapsed())
        .unwrap_or_default();
    let time_to_first_token = time_to_first_token.unwrap_or_else(|| started.elapsed());
    let thinking = strip_think_tags(&thinking);
    let content = if grammar.is_some() {
        json_payload(&content).to_string()
    } else {
        content
    };
    Ok(LlmResponse {
        thinking,
        content,
        prompt_tokens,
        thinking_tokens,
        generated_tokens: produced,
        time_to_first_token,
        duration,
    })
}

fn piece_from(
    model: &LlamaModel,
    decoder: &mut encoding_rs::Decoder,
    token: LlamaToken,
) -> Result<String> {
    Ok(model.token_to_piece(token, decoder, true, None)?)
}

fn push_piece(
    produced: &mut usize,
    started: Instant,
    time_to_first_token: &mut Option<Duration>,
    decode_started: &mut Option<Instant>,
) {
    if time_to_first_token.is_none() {
        *time_to_first_token = Some(started.elapsed());
        *decode_started = Some(Instant::now());
    }
    *produced += 1;
}

fn next_token(
    sampler: &mut LlamaSampler,
    ctx: &llama_cpp_2::context::LlamaContext<'_>,
    batch: &LlamaBatch,
    constrained: bool,
) -> Result<LlamaToken> {
    let idx = batch.n_tokens() - 1;
    if !constrained {
        return Ok(sampler.sample(ctx, idx));
    }
    // llama-cpp-2 0.1.156's sampler.sample() aborts in llama-grammar.cpp when this
    // grammar reaches an empty stack. Selecting from explicitly filtered candidates
    // avoids the native GGML_ASSERT and lets us return an error instead of SIGABRT.
    let mut candidates = ctx.token_data_array_ith(idx);
    sampler.apply(&mut candidates);
    candidates
        .selected_token()
        .ok_or_else(|| anyhow!("grammar sampler failed to select a token"))
}

fn open_thinking(formatted: &str) -> String {
    for closed in [
        "<think>\n\n</think>\n\n",
        "<think>\n</think>\n\n",
        "<think></think>",
    ] {
        if let Some(index) = formatted.rfind(closed) {
            return format!("{}<think>\n", &formatted[..index]);
        }
    }
    if formatted.contains("<think>") && !formatted.contains("</think>") {
        return formatted.to_string();
    }
    format!("{formatted}<think>\n")
}

fn strip_think_tags(text: &str) -> String {
    text.replace("<think>", "")
        .replace("</think>", "")
        .trim()
        .to_string()
}

fn json_payload(output: &str) -> &str {
    let from = output
        .rfind("</think>")
        .map(|index| index + "</think>".len())
        .unwrap_or(0);
    let rest = &output[from..];
    let Some(start) = rest.find('{') else {
        return rest.trim();
    };
    rest[start..].trim()
}

pub(crate) fn backend() -> Result<&'static LlamaBackend> {
    static BACKEND: OnceLock<LlamaBackend> = OnceLock::new();
    if let Some(backend) = BACKEND.get() {
        return Ok(backend);
    }
    let log_options = LogOptions::default().with_logs_enabled(false);
    send_logs_to_tracing(log_options);
    let backend = LlamaBackend::init()?;
    Ok(BACKEND.get_or_init(|| backend))
}

pub(crate) fn download_model(options: &LlmOptions) -> Result<PathBuf> {
    fs::create_dir_all(HF_HOME)?;
    let cache = PathBuf::from(HF_HOME);
    let cache = if cache.is_absolute() {
        cache
    } else {
        std::env::current_dir()?.join(cache)
    };
    let total_ram = total_ram_bytes()?;
    let model = select_model(total_ram, options.min_params_b);
    let repo = model.repo.to_string();
    Ok(ApiBuilder::new()
        .with_progress(true)
        .with_cache_dir(cache)
        .build()?
        .model(repo)
        .get(model.file)?)
}

fn select_model(total_ram: u64, min_params_b: f32) -> &'static GgufModel {
    let budget = total_ram / 5;
    MODELS
        .iter()
        .filter(|model| model.params_b >= min_params_b)
        .rev()
        .find(|model| model.estimated_ram <= budget)
        .or_else(|| MODELS.iter().find(|model| model.params_b >= min_params_b))
        .unwrap_or(&MODELS[0])
}

fn total_ram_bytes() -> Result<u64> {
    #[cfg(target_os = "linux")]
    {
        let meminfo = fs::read_to_string("/proc/meminfo")?;
        for line in meminfo.lines() {
            let Some(rest) = line.strip_prefix("MemTotal:") else {
                continue;
            };
            let Some(value) = rest.split_whitespace().next() else {
                bail!("MemTotal missing value");
            };
            let kb: u64 = value.parse()?;
            return Ok(kb.saturating_mul(1024));
        }
        bail!("MemTotal missing from /proc/meminfo")
    }
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()?;
        if !output.status.success() {
            bail!("sysctl hw.memsize failed");
        }
        let value = String::from_utf8(output.stdout)?;
        Ok(value.trim().parse()?)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        Ok(8 * 1024 * 1024 * 1024)
    }
}
