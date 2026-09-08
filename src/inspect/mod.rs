use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::llamacpp::{self, LlmOptions, LlmResponse};

pub mod occupations;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub kind: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceSpan {
    pub start: usize,
    pub end: usize,
    pub kind: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingsArtifact {
    pub document_sha256: String,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone)]
pub struct Inspection {
    pub findings: Vec<Finding>,
    pub generations: Vec<LlmResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct InspectionFindings {
    findings: Vec<Finding>,
}

pub fn inspect(
    text: &str,
    prompt: &str,
    identifiers: &[&str],
    options: &LlmOptions,
) -> Result<Inspection> {
    if identifiers.is_empty() {
        bail!("inspect requires at least one identifier");
    }

    let schema = findings_schema(identifiers).to_string();
    let prompt = inspection_prompt(text, prompt, identifiers);
    let response = llamacpp::complete_json(&prompt, &schema, options)?;
    let parsed: InspectionFindings = serde_json::from_str(&response.content)
        .with_context(|| format!("inspect json: {:?}", response.content))?;
    Ok(Inspection {
        findings: parsed.findings,
        generations: vec![response],
    })
}

pub fn document_sha256(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

pub fn evidence_spans(text: &str, findings: &[Finding]) -> Vec<EvidenceSpan> {
    let mut spans = Vec::new();
    for finding in findings {
        for evidence in &finding.evidence {
            if evidence.is_empty() {
                continue;
            }
            for (start, matched) in text.match_indices(evidence) {
                spans.push(EvidenceSpan {
                    start,
                    end: start + matched.len(),
                    kind: finding.kind.clone(),
                    text: matched.to_string(),
                });
            }
        }
    }
    spans
}

pub fn write_artifact(path: &Path, text: &str, findings: &[Finding]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let artifact = FindingsArtifact {
        document_sha256: document_sha256(text),
        findings: findings.to_vec(),
    };
    let json = serde_json::to_string_pretty(&artifact)?;
    fs::write(path, json)?;
    Ok(())
}

pub fn load_for_document(path: &Path, text: &str) -> Result<Vec<Finding>> {
    let json = fs::read_to_string(path)?;
    let artifact: FindingsArtifact = serde_json::from_str(&json)
        .with_context(|| format!("findings json: {}", path.display()))?;
    let expected_sha256 = document_sha256(text);
    if artifact.document_sha256 != expected_sha256 {
        bail!("findings fingerprint does not match the input document");
    }
    Ok(artifact.findings)
}

fn inspection_prompt(text: &str, prompt: &str, identifiers: &[&str]) -> String {
    let identifiers = identifiers.join(", ");
    format!(
        "{prompt}\nKinds: {identifiers}\nKinds are categories to extract, not words to search for.\nCopy every matching span exactly from the input.\n\nInput:\n{text}"
    )
}

fn findings_schema(identifiers: &[&str]) -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "findings": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "kind": { "enum": identifiers },
                        "evidence": {
                            "type": "array",
                            "items": { "type": "string" }
                        }
                    },
                    "required": ["kind", "evidence"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["findings"],
        "additionalProperties": false
    })
}
