use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Result, anyhow, bail};
use gaze::token_shape;
use gaze::{
    Action, CleanDocument, Context, LocaleChain, PiiClass, Pipeline, Policy, RawDocument, RuleSpec,
    Rulepack, RulepackSource, Scope, SensitiveSnapshot, Session,
};
use gaze_assembly::build_pipeline;

const DEFAULT_SCOPE: &str = "mask";

pub(crate) struct Gaze {
    pipeline: Pipeline,
    locale_chain: LocaleChain,
}

impl Gaze {
    pub fn open(labels: &[&str]) -> Result<Self> {
        let contents =
            gaze_recognizers::embedded("core").ok_or_else(|| anyhow!("missing core rulepack"))?;
        let rulepack = Rulepack::load(RulepackSource::Embedded(contents))?;
        let locale_chain = LocaleChain::merge_cli_policy_rulepack_default(
            None,
            None,
            Some(&rulepack.default_locales),
        );
        let mut policy = Policy::default();
        policy.rules = tokenize_rules(labels);
        let context = Context {
            dictionaries: HashMap::new(),
            class_map: HashMap::new(),
            fields: serde_json::Map::new(),
        };
        let pipeline = build_pipeline(&policy, &context, &[rulepack], &locale_chain, None)?;
        Ok(Self {
            pipeline,
            locale_chain,
        })
    }

    pub fn encode(&self, session: &Session, text: String) -> Result<String> {
        let locales = self.locale_chain.as_slice();
        match self
            .pipeline
            .pseudonymize_with_context(session, RawDocument::Text(text), locales)?
        {
            CleanDocument::Text(clean) => Ok(clean),
            _ => bail!("gaze-pii returned a non-text document"),
        }
    }

    pub fn decode(session: &Session, text: &str) -> Result<String> {
        Ok(session.restore_strict_text(text)?)
    }

    pub fn labels(text: &str) -> Vec<String> {
        token_shape::find_tokens(text).map(str::to_string).collect()
    }

    pub fn load_or_create_session(snapshot: &Path) -> Result<Session> {
        if snapshot.exists() {
            return Self::import_session(snapshot);
        }
        Ok(Session::new(Scope::Conversation(DEFAULT_SCOPE.into()))?)
    }

    pub fn import_session(snapshot: &Path) -> Result<Session> {
        let bytes = fs::read(snapshot)?;
        let snapshot = SensitiveSnapshot::from(bytes);
        Ok(Session::import(snapshot)?)
    }

    pub fn persist_session(snapshot: &Path, session: &Session) -> Result<()> {
        if let Some(parent) = snapshot.parent() {
            fs::create_dir_all(parent)?;
        }
        let exported = session.export()?;
        let bytes = exported.into_bytes();
        fs::write(snapshot, bytes)?;
        Ok(())
    }
}

pub(crate) fn pii_class(label: &str) -> PiiClass {
    match label.to_ascii_lowercase().as_str() {
        "person" | "name" => PiiClass::Name,
        "organization" | "org" => PiiClass::Organization,
        "location" => PiiClass::Location,
        "email" => PiiClass::Email,
        other => PiiClass::from_policy_name(other).unwrap_or_else(|| PiiClass::custom(other)),
    }
}

fn tokenize_rules(labels: &[&str]) -> Vec<RuleSpec> {
    let mut rules: Vec<RuleSpec> = labels
        .iter()
        .map(|label| RuleSpec::Class {
            class: pii_class(label),
            action: Action::Tokenize,
        })
        .collect();
    rules.push(RuleSpec::Default {
        action: Action::Preserve,
    });
    rules
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_preserves_existing_token_and_tokenizes_remaining_pii() {
        let session = Session::new(Scope::Conversation("test".into())).expect("create session");
        let name = session
            .tokenize(&PiiClass::Name, "Priya Nair")
            .expect("tokenize name");
        let gaze = Gaze::open(&["email"]).expect("open gaze");
        let input = format!("{name} emailed priya@example.com");
        let encoded = gaze.encode(&session, input).expect("encode text");

        assert!(encoded.contains(&name), "existing token changed: {encoded}");
        assert_eq!(Gaze::labels(&encoded).len(), 2);
        assert!(
            !encoded.contains("priya@example.com"),
            "email was not tokenized: {encoded}"
        );
        assert_eq!(
            Gaze::decode(&session, &encoded).expect("decode text"),
            "Priya Nair emailed priya@example.com"
        );
    }
}
