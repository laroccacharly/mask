mod common;

use mask::llamacpp;
use mask::{TokenFormatter, Vault};

const PERSON: &str = "Priya Nair";
const PERSON_CAPS: &str = "PRIYA NAIR";
const FOLDED_PERSON: &str = "priya nair";

const INTERVIEW_NOTES: &str = "\
Northwind Logistics — discovery interview
Date: 12 March 2026
Interviewer: Jonah Hale, Aperture Consulting
Client attendee: Priya Nair, Director of Operations, Northwind Logistics

Priya Nair said warehouse overtime in Cleveland is the main cost driver.
She asked Aperture for a six-week diagnostic with a steering readout on 24 April 2026.
You must send the Affiliate a copy of the readout. Do not email the customer.
";

const KICKOFF_MEMO: &str = "\
Internal kickoff memo
From: Elena Voss, Aperture Consulting
Re: Northwind Logistics diagnostic

PRIYA NAIR is the day-to-day client owner for this engagement.
Weekly steering calls include Priya Nair; do not copy other Northwind staff until she approves.
";

#[test]
fn shared_person_keeps_one_label_through_llm_roundtrip() {
    let dir = tempfile::tempdir().expect("temp dir");
    let vault = Vault::open(dir.path().join("gaze.snapshot"))
        .expect("open vault")
        .with_labels(&["person"])
        .expect("configure labels")
        .with_token_formatter(TokenFormatter);

    let encoded_notes = common::timed("encode interview notes", || {
        vault
            .encode(INTERVIEW_NOTES)
            .expect("encode interview notes")
    });
    let encoded_memo = common::timed("encode kickoff memo", || {
        vault.encode(KICKOFF_MEMO).expect("encode kickoff memo")
    });

    assert!(
        !encoded_notes.contains(PERSON),
        "encoded notes still contain {PERSON}: {encoded_notes}"
    );
    assert!(
        !encoded_memo.contains(PERSON),
        "encoded memo still contain {PERSON}: {encoded_memo}"
    );
    assert!(
        !encoded_memo.contains(PERSON_CAPS),
        "encoded memo still contain {PERSON_CAPS}: {encoded_memo}"
    );
    assert!(
        encoded_notes.contains("You")
            && encoded_notes.contains("Affiliate")
            && encoded_notes.contains("Director")
            && encoded_notes.contains("customer"),
        "rejected contract words were tokenized: {encoded_notes}"
    );

    let shared_labels: Vec<String> = vault
        .labels(&encoded_notes)
        .into_iter()
        .filter(|label| encoded_memo.contains(label.as_str()))
        .collect();
    assert!(
        !shared_labels.is_empty(),
        "expected {PERSON} to encode to the same label in both documents\nnotes: {encoded_notes}\nmemo: {encoded_memo}"
    );

    let owner_line = encoded_memo
        .lines()
        .find(|line| line.contains("day-to-day"))
        .expect("owner line");
    let calls_line = encoded_memo
        .lines()
        .find(|line| line.contains("Weekly steering"))
        .expect("calls line");
    assert!(
        vault
            .labels(owner_line)
            .iter()
            .any(|label| calls_line.contains(label.as_str())),
        "expected {PERSON} and {PERSON_CAPS} to share one label\nmemo: {encoded_memo}"
    );

    let prompt = format!(
        "You are assisting a consultant. Use only the two documents below.\n\
         Copy names exactly as they appear. Do not invent names.\n\n\
         Document 1:\n{encoded_notes}\n\n\
         Document 2:\n{encoded_memo}\n\n\
         Question: Who is the Director of Operations at Northwind Logistics?\n\
         Reply with only that person's name."
    );

    let reply = common::timed("llamacpp", || llamacpp::complete(&prompt).expect("run llm"));

    assert!(
        !reply.contains(PERSON),
        "llm leaked the real name {PERSON}: {reply}"
    );
    assert!(
        shared_labels.iter().any(|label| reply.contains(label)),
        "llm reply should use the encoded person label {shared_labels:?}, got {reply}"
    );

    let decoded = vault.decode(&reply).expect("decode llm reply");
    assert!(
        decoded.contains(FOLDED_PERSON),
        "decoded llm reply should restore {FOLDED_PERSON}, got {decoded}"
    );
}
