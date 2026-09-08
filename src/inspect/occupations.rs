use anyhow::Result;

use crate::inspect::{self, Inspection};
use crate::llamacpp::LlmOptions;

pub const KIND: &str = "occupation";
pub const KINDS: &[&str] = &[KIND];
pub const ARTIFACT: &str = "data/occupations.json";
pub const PROMPT: &str = "\
Find every instance of the given identifier kinds. \
A kind names a category such as occupation, not a search term. \
Copy evidence exactly from the input. Report every match.
";

pub fn inspect(text: &str) -> Result<Inspection> {
    inspect::inspect(
        text,
        PROMPT,
        KINDS,
        &LlmOptions {
            max_new_tokens: 1536,
            max_think_tokens: 768,
            ctx_size: 2048,
            min_params_b: 2.0,
        },
    )
}
