use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Parser, Subcommand};
use mask::{TokenFormatter, Vault, cuad, inspect, pdf, serve, vulkan};

#[derive(Parser)]
#[command(name = "mask", about = "Mask CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Benchmark llama.cpp on CPU and Vulkan
    Vulkan {
        /// Number of tokens to generate per run
        #[arg(short, long, default_value_t = 128)]
        tokens: usize,
        /// Prompt used for both runs
        #[arg(
            short,
            long,
            default_value = "Write a concise explanation of why the sky is blue."
        )]
        prompt: String,
    },
    /// CUAD dataset commands
    Cuad {
        #[command(subcommand)]
        command: CuadCommands,
    },
    /// PDF commands
    Pdf {
        #[command(subcommand)]
        command: PdfCommands,
    },
    /// Vault commands
    Vault {
        #[command(subcommand)]
        command: VaultCommands,
    },
    /// Replace PII with gaze-pii tokens and persist the restore map
    Encode {
        /// Input text path
        #[arg(short, long)]
        input: PathBuf,
        /// Output text path
        #[arg(short, long)]
        output: PathBuf,
        /// Gaze session snapshot path
        #[arg(short, long, default_value = "data/gaze.snapshot")]
        snapshot: PathBuf,
        /// Classes to redact
        #[arg(long = "label", default_values = ["person", "organization", "location", "email"])]
        labels: Vec<String>,
        /// Inspection findings artifact
        #[arg(long)]
        findings: Option<PathBuf>,
    },
    /// Inspect text with a local LLM
    Inspect {
        #[command(subcommand)]
        command: InspectCommands,
    },
    /// Start the HTTP server in the foreground
    Serve {
        /// Port to listen on
        #[arg(short, long, default_value_t = 8000)]
        port: u16,
    },
    /// Restore original values from a gaze-pii session snapshot
    Decode {
        /// Input text path
        #[arg(short, long)]
        input: PathBuf,
        /// Output text path
        #[arg(short, long)]
        output: PathBuf,
        /// Gaze session snapshot path
        #[arg(short, long, default_value = "data/gaze.snapshot")]
        snapshot: PathBuf,
        /// Classes to redact
        #[arg(long = "label", default_values = ["person", "organization", "location", "email"])]
        labels: Vec<String>,
    },
}

#[derive(Subcommand)]
enum InspectCommands {
    /// Find occupations
    Occupations {
        /// Input text path
        #[arg(short, long)]
        input: PathBuf,
        /// Findings artifact path
        #[arg(short, long, default_value = inspect::occupations::ARTIFACT)]
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum VaultCommands {
    /// Delete the snapshot and create a new empty session
    Reset {
        /// Gaze session snapshot path
        #[arg(short, long, default_value = "data/gaze.snapshot")]
        snapshot: PathBuf,
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum CuadCommands {
    /// Download and extract the CUAD dataset
    Setup,
}

#[derive(Subcommand)]
enum PdfCommands {
    /// Extract text from a PDF
    Extract {
        /// Input PDF path
        #[arg(short, long)]
        input: PathBuf,
        /// Output text path
        #[arg(short, long)]
        output: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Vulkan { tokens, prompt } => vulkan::benchmark(&prompt, tokens),
        Commands::Vault { command } => match command {
            VaultCommands::Reset { snapshot, yes } => reset_vault(&snapshot, yes),
        },
        Commands::Cuad { command } => match command {
            CuadCommands::Setup => cuad::setup(),
        },
        Commands::Pdf { command } => match command {
            PdfCommands::Extract { input, output } => pdf::extract(&input, &output),
        },
        Commands::Serve { port } => serve::run(port),
        Commands::Inspect { command } => match command {
            InspectCommands::Occupations { input, output } => {
                let text = fs::read_to_string(&input)?;
                let inspection = inspect::occupations::inspect(&text)?;
                inspect::write_artifact(&output, &text, &inspection.findings)?;
                println!(
                    "Wrote {} findings to {}",
                    inspection.findings.len(),
                    output.display()
                );
                Ok(())
            }
        },
        Commands::Encode {
            input,
            output,
            snapshot,
            labels,
            findings,
        } => {
            let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
            let vault = Vault::open(&snapshot)?
                .with_labels(&labels)?
                .with_token_formatter(TokenFormatter);
            let text = fs::read_to_string(&input)?;
            let findings = match findings {
                Some(path) => inspect::load_for_document(&path, &text)?,
                None => Vec::new(),
            };
            let encoded = vault.encode_with_findings(&text, &findings)?;
            write_text(&output, &encoded)
        }
        Commands::Decode {
            input,
            output,
            snapshot,
            labels,
        } => {
            let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
            let vault = Vault::open(&snapshot)?
                .with_labels(&labels)?
                .with_token_formatter(TokenFormatter);
            let text = fs::read_to_string(&input)?;
            let decoded = vault.decode(&text)?;
            write_text(&output, &decoded)
        }
    }
}

fn reset_vault(snapshot: &Path, yes: bool) -> Result<()> {
    if !yes && !confirm_reset(snapshot)? {
        println!("Aborted.");
        return Ok(());
    }
    Vault::reset(snapshot)?;
    println!("Reset vault at {}", snapshot.display());
    Ok(())
}

fn confirm_reset(snapshot: &Path) -> Result<bool> {
    print!(
        "This will delete {} and create a new vault. Continue? [y/N] ",
        snapshot.display()
    );
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(matches!(
        line.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

fn write_text(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)?;
    Ok(())
}
