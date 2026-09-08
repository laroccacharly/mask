mod common;

use mask::inspect::Finding;
use mask::inspect::occupations;

const NOTES: &str = "\
Weekly clinic follow-up, 4 April 2026
Attendees: Jonah Hale, Elena Park, warehouse leads from Cleveland.

The overtime report is still the main cost driver. Jonah asked for a six-week diagnostic
and a steering readout on 24 April. Send the Affiliate a copy. Do not email the customer.
Jonah Hale is a labor economist.

Elena joined late because of weather in Boise. She walked through bed capacity, on-call
coverage, and the stalled Northwind acquisition. Dr. Elena Park is a pediatric neurosurgeon.
The group agreed to keep Project Cedar off the written agenda until legal reviews the draft.

Action items: Hale will send the readout template. Park will confirm next week's room.
";

fn occupation_quotes(findings: &[Finding]) -> Vec<&str> {
    findings
        .iter()
        .filter(|finding| finding.kind == occupations::KIND)
        .flat_map(|finding| finding.evidence.iter().map(String::as_str))
        .collect()
}

fn assert_occupation_span(quotes: &[&str], phrase: &str) {
    let quote = quotes
        .iter()
        .copied()
        .find(|item| item.contains(phrase))
        .unwrap_or_else(|| panic!("expected occupation {phrase}, got {quotes:?}"));

    assert!(
        NOTES.contains(quote),
        "evidence is not copied from the input: {quote:?}"
    );
    assert_ne!(
        quote, NOTES,
        "evidence should be a span, not the whole input: {quote:?}"
    );
    assert!(
        quote.len() < NOTES.len() / 4,
        "evidence is too long to be a useful span: {quote:?}"
    );
}

#[test]
fn classifies_contextual_identifiers() {
    let inspection = common::timed("inspect occupations", || {
        occupations::inspect(NOTES).expect("inspect occupations")
    });
    let quotes = occupation_quotes(&inspection.findings);
    for (index, generation) in inspection.generations.iter().enumerate() {
        eprintln!("generation {index}\n{generation}");
    }

    assert_occupation_span(&quotes, "labor economist");
    assert_occupation_span(&quotes, "pediatric neurosurgeon");
}
