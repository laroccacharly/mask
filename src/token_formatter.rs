use std::collections::HashMap;

use gaze::{Session, token_shape};

pub struct TokenFormatter;

impl TokenFormatter {
    pub(crate) fn casefold(raw: &str) -> String {
        raw.to_lowercase()
    }

    pub(crate) fn format(&self, session: &Session, text: &str) -> String {
        text.replace(&format!("<{}:", session.session_hex()), "<")
    }

    pub(crate) fn unformat(&self, session: &Session, text: &str) -> String {
        let prefix = format!("<{}:", session.session_hex());
        let mut by_label = HashMap::new();
        for entry in session.snapshot_entries() {
            let Some(rest) = entry.token.strip_prefix(&prefix) else {
                continue;
            };
            let Some(inner) = rest.strip_suffix('>') else {
                continue;
            };
            let full_key = entry.token.clone();
            let full_value = entry.token.clone();
            by_label.insert(full_key, full_value);
            let compact_key = format!("<{inner}>");
            let compact_value = entry.token.clone();
            by_label.insert(compact_key, compact_value);
            let inner_key = inner.to_string();
            let inner_value = entry.token.clone();
            by_label.insert(inner_key, inner_value);
        }

        let matches: Vec<(usize, usize, String)> = token_shape::pattern()
            .find_iter(text)
            .map(|matched| (matched.start(), matched.end(), matched.as_str().to_string()))
            .collect();
        let mut restored = text.to_string();
        for (start, end, label) in matches.into_iter().rev() {
            let Some(full) = by_label.get(&label) else {
                continue;
            };
            restored.replace_range(start..end, full);
        }
        restored
    }
}

#[cfg(test)]
mod tests {
    use gaze::{PiiClass, Scope};

    use super::*;

    #[test]
    fn compact_tokens_round_trip() {
        let session = Session::new(Scope::Conversation("test".into())).expect("create session");
        let token = session
            .tokenize(&PiiClass::Name, "Priya Nair")
            .expect("tokenize name");
        let formatter = TokenFormatter;

        let formatted = formatter.format(&session, &token);
        let unformatted = formatter.unformat(&session, &formatted);

        assert_eq!(formatted, "<Name_1>");
        assert_eq!(unformatted, token);
        assert_eq!(
            session.restore_strict_text(&unformatted).expect("restore"),
            "Priya Nair"
        );
    }
}
