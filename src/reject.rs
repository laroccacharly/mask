const REJECTED: &[&str] = &[
    "you",
    "your",
    "affiliate",
    "affiliates",
    "customer",
    "customers",
    "applicant",
    "applicants",
    "person",
    "persons",
    "person's",
    "officer",
    "officers",
    "director",
    "directors",
    "employee",
    "employees",
    "shareholder",
    "shareholders",
    "website",
    "websites",
    "webpage",
    "page",
    "email",
    "e-mail",
    "link",
    "links",
];

pub fn is_rejected(raw: &str) -> bool {
    let key = normalize(raw);
    let key = key.as_str();
    REJECTED.contains(&key)
}

fn normalize(raw: &str) -> String {
    raw.replace('\u{00a0}', " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
