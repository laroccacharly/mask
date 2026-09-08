use mask::gliner::{self, Mention};

fn has_mention(mentions: &[Mention], field: &str, text: &str) -> bool {
    mentions
        .iter()
        .any(|mention| mention.field == field && mention.text.contains(text))
}

#[test]
fn detects_named_entities() {
    let mut engine = gliner::load_engine().expect("load GLiNER 2.5");

    let agreement = "This Agreement is made on October 12, 2024, by Marie Tremblay, VP of Engineering at Acme Robotics Corp in Montreal, Canada.";
    let agreement_mentions = gliner::extract_entities(
        &mut engine,
        agreement,
        &["person", "organization", "location", "date", "job title"],
    )
    .expect("extract agreement entities");

    assert!(
        has_mention(&agreement_mentions, "person", "Marie Tremblay"),
        "expected person Marie Tremblay, got {agreement_mentions:?}"
    );
    assert!(
        has_mention(&agreement_mentions, "organization", "Acme Robotics"),
        "expected organization Acme Robotics, got {agreement_mentions:?}"
    );
    assert!(
        has_mention(&agreement_mentions, "location", "Montreal"),
        "expected location Montreal, got {agreement_mentions:?}"
    );

    let notice =
        "Notices to the Affiliate shall be sent to Giuseppe Verdi at g.verdi@example.it, Milano.";
    let notice_mentions =
        gliner::extract_entities(&mut engine, notice, &["person", "email", "location"])
            .expect("extract notice entities");

    assert!(
        has_mention(&notice_mentions, "person", "Giuseppe Verdi"),
        "expected person Giuseppe Verdi, got {notice_mentions:?}"
    );
    assert!(
        has_mention(&notice_mentions, "email", "g.verdi@example.it"),
        "expected email g.verdi@example.it, got {notice_mentions:?}"
    );
    assert!(
        has_mention(&notice_mentions, "location", "Milano"),
        "expected location Milano, got {notice_mentions:?}"
    );
}
