use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use anyhow::{Result, anyhow};
use serde::Serialize;

use crate::gaze::Gaze;
use crate::gliner::{self, BoundaryEngine};
use crate::inspect::Finding;
use crate::token_formatter::TokenFormatter;

pub const DEFAULT_SNAPSHOT: &str = "data/gaze.snapshot";

#[derive(Clone, Debug, Serialize)]
pub struct VaultMapping {
    pub token: String,
    pub value: String,
    pub class: String,
}

pub struct Vault {
    snapshot: PathBuf,
    gaze: Gaze,
    engine: Mutex<BoundaryEngine>,
    labels: Vec<String>,
    token_formatter: Option<TokenFormatter>,
}

impl Vault {
    pub fn reset(snapshot: impl AsRef<Path>) -> Result<()> {
        let snapshot = snapshot.as_ref();
        if snapshot.exists() {
            fs::remove_file(snapshot)?;
        }
        let session = Gaze::load_or_create_session(snapshot)?;
        Gaze::persist_session(snapshot, &session)
    }

    pub fn open(snapshot: impl AsRef<Path>) -> Result<Self> {
        let gaze = Gaze::open(&[])?;
        let engine = gliner::load_engine()?;
        Ok(Self {
            snapshot: snapshot.as_ref().to_path_buf(),
            gaze,
            engine: Mutex::new(engine),
            labels: Vec::new(),
            token_formatter: None,
        })
    }

    pub fn with_labels(mut self, labels: &[&str]) -> Result<Self> {
        self.gaze = Gaze::open(labels)?;
        self.labels = labels.iter().map(|label| (*label).to_string()).collect();
        Ok(self)
    }

    pub fn with_token_formatter(mut self, token_formatter: TokenFormatter) -> Self {
        self.token_formatter = Some(token_formatter);
        self
    }

    pub fn encode(&self, text: &str) -> Result<String> {
        self.encode_with_findings(text, &[])
    }

    pub fn encode_with_findings(&self, text: &str, findings: &[Finding]) -> Result<String> {
        let session = Gaze::load_or_create_session(&self.snapshot)?;
        let mut engine = self
            .engine
            .lock()
            .map_err(|_| anyhow!("gliner engine lock poisoned"))?;
        let labels: Vec<&str> = self.labels.iter().map(String::as_str).collect();
        let extra = crate::inspect::evidence_spans(text, findings)
            .into_iter()
            .map(|span| (span.start, span.end, span.kind, span.text))
            .collect();
        let text = gliner::tokenize_with_extra(&mut engine, &session, text, &labels, extra)?;
        drop(engine);
        let clean = self.gaze.encode(&session, text)?;
        Gaze::persist_session(&self.snapshot, &session)?;
        let clean = if let Some(token_formatter) = &self.token_formatter {
            token_formatter.format(&session, &clean)
        } else {
            clean
        };
        Ok(clean)
    }

    pub fn decode(&self, text: &str) -> Result<String> {
        let session = Gaze::import_session(&self.snapshot)?;
        let text = if let Some(token_formatter) = &self.token_formatter {
            token_formatter.unformat(&session, text)
        } else {
            text.to_string()
        };
        Gaze::decode(&session, &text)
    }

    pub fn labels(&self, text: &str) -> Vec<String> {
        Gaze::labels(text)
    }

    pub fn mappings(&self) -> Result<Vec<VaultMapping>> {
        if !self.snapshot.exists() {
            return Ok(Vec::new());
        }
        let session = Gaze::import_session(&self.snapshot)?;
        let mut mappings: Vec<VaultMapping> = session
            .snapshot_entries()
            .into_iter()
            .map(|entry| {
                let token = match &self.token_formatter {
                    Some(formatter) => formatter.format(&session, &entry.token),
                    None => entry.token,
                };
                VaultMapping {
                    class: entry.class.class_name(),
                    token,
                    value: entry.raw,
                }
            })
            .collect();
        mappings.sort_by(|left, right| {
            left.class
                .cmp(&right.class)
                .then_with(|| left.token.cmp(&right.token))
                .then_with(|| left.value.cmp(&right.value))
        });
        Ok(mappings)
    }

    pub fn reset_snapshot(&self) -> Result<()> {
        Self::reset(&self.snapshot)
    }
}
