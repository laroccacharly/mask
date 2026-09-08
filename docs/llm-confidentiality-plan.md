# Local LLM Confidentiality Inspection Plan

## Goal

Add an optional local-LLM inspection step that finds quasi-identifiers and contextual confidential information missed by Gaze and GLiNER. Preserve the existing fast `encode` behavior and keep all document modification inside the existing tokenization pipeline.

The LLM only proposes findings. It never rewrites the document or creates replacement tokens.

## User workflows

Keep standard encoding unchanged:

```console
mask encode --input document.txt --output encoded.txt
```

Add a reviewable inspection workflow:

```console
mask inspect --input document.txt --output data/findings.json
mask encode --input document.txt --findings data/findings.json --output encoded.txt
```

After the reviewable workflow is stable, add an optional convenience mode:

```console
mask encode --deep --input document.txt --output encoded.txt
```

`--deep` runs inspection and encoding together. It is never enabled by default.

## Pipeline

Run every detector against the original document, combine and validate their spans, and tokenize once:

```text
original document
  ├── Gaze findings
  ├── GLiNER findings
  └── optional local-LLM findings
              │
              ▼
      validate and merge spans
              │
              ▼
       existing tokenization
              │
              ▼
        encoded document
```

Do not run the LLM as a rewriter after standard encoding. That would complicate offsets and restoration and could remove context needed to recognize quasi-identifiers.

## Initial taxonomy

Restrict the model to a small fixed set:

- `demographic`: precise age, unusual family status, nationality, or distinctive background
- `occupation`: rare role, specialty, team, seniority, or employer relationship
- `location`: small location, workplace location, or unusual travel pattern
- `date_or_event`: distinctive date, incident, attendance, or milestone
- `affiliation`: school, client, project, department, club, or organizational relationship
- `internal_secret`: code name, transaction, strategy, or other non-public business fact
- `other`: clearly sensitive contextual information that does not fit another kind

Direct names, email addresses, phone numbers, and similar PII remain primarily the responsibility of Gaze and GLiNER.

## Model output

Ask the model for minimal JSON containing exact quotes from the supplied chunk:

```json
{
  "findings": [
    {
      "kind": "demographic",
      "evidence": [
        "47-year-old",
        "pediatric neurosurgeon",
        "recently moved from Boise"
      ]
    },
    {
      "kind": "internal_secret",
      "evidence": [
        "Project Cedar",
        "planned acquisition of Northwind"
      ]
    }
  ]
}
```

An evidence array represents details whose combination may create the disclosure risk. Each evidence value must be an exact substring of the input chunk.

Do not request offsets, replacement tokens, rewritten text, free-form labels, confidence scores, or long explanations. The application computes offsets from exact evidence strings and rejects output that cannot be matched.

The persisted artifact should be enriched by the application with:

- schema, model, prompt, and chunking versions
- a document fingerprint
- chunk IDs and document offsets
- review status for each finding

Store findings under `data/` because they contain sensitive source text.

## Prompt outline

Use a short instruction with two or three examples, including an empty result:

```text
Find contextual information that could identify a person, organization,
client, case, or confidential activity.

Focus on unusual combinations such as a precise age, rare occupation,
small location, specific event, affiliation, project name, or non-public
business fact. Direct identifiers may already be handled elsewhere.

Return only JSON matching the schema.

Rules:
1. Copy each evidence string exactly from the input.
2. Use the shortest meaningful evidence string.
3. Group evidence when its combination creates the risk.
4. Do not report ordinary generic information or existing mask tokens.
5. Do not rewrite the input.
6. Return {"findings":[]} when nothing qualifies.
```

Use deterministic generation and constrained JSON/schema decoding when supported by the local inference backend.

## Chunking

1. Split on paragraphs.
2. Split oversized paragraphs on sentence boundaries.
3. Use token-based hard splitting only as a fallback.
4. Add one or two sentences of overlap.
5. Assign stable chunk IDs and ownership boundaries.

For the current 2,048-token context, begin with approximately 700–900 input tokens per chunk. Leave room for instructions, examples, and output.

For each response:

- parse and validate the JSON schema
- reject evidence not copied exactly from its chunk
- locate evidence in the original document
- ignore matches outside the chunk's ownership boundary
- deduplicate findings caused by overlap
- merge overlapping spans using deterministic rules

Initially, apply an accepted evidence string to every exact occurrence within the chunk's ownership region. Add left/right anchors later only if duplicate phrases cause practical ambiguity.

Document-wide combinations are out of scope for the first version. A later optional second pass can give the model only the deduplicated evidence candidates from all chunks and ask it to group cross-chunk combinations.

## Failure behavior

- Retry malformed model output once with a minimal JSON-correction instruction.
- Mark a chunk incomplete if the retry fails.
- Never silently describe a partially inspected document as fully inspected.
- Allow completed chunks to be resumed from the findings artifact.
- Cache results by document fingerprint, model, prompt version, and inspection settings.
- Never apply findings when the artifact fingerprint does not match the input document.

## Implementation milestones

### 1. Inspection domain model

- Define the findings schema and fixed taxonomy.
- Add parsing, validation, exact-match offset resolution, deduplication, and overlap tests.
- Define versioned artifact serialization and document fingerprinting.

### 2. Chunking

- Implement paragraph/sentence-aware token-budget chunking.
- Track overlap and ownership boundaries.
- Test Unicode offsets, repeated text, long paragraphs, and boundary-spanning sentences.

### 3. Local model integration

- Add the inspection prompt and constrained structured generation.
- Load the model once per inspection rather than once per chunk.
- Add progress, cancellation, malformed-output retry, and resumability.

### 4. CLI review workflow

- Add `mask inspect` and write artifacts only beneath `data/` by default.
- Add `encode --findings` with fingerprint and schema validation.
- Provide a readable summary of accepted, rejected, and incomplete findings.

### 5. Unified encoding

- Combine Gaze, GLiNER, and accepted LLM spans before tokenization.
- Map LLM kinds to stable custom PII classes such as `quasi_demographic` and `internal_secret`.
- Define and test deterministic overlap precedence.

### 6. Convenience and UI

- Add `encode --deep` after the artifact workflow is reliable.
- Add Standard, Deep, and Inspect/Review choices to the UI.
- Show estimated cost, chunk progress, cancellation, incomplete analysis, and individual finding approval.

### 7. Evaluation

- Build a small representative corpus of quasi-identification and confidential-context examples.
- Measure document-level recall, false positives, runtime, and the percentage of useful text unnecessarily masked.
- Compare model sizes and chunk settings before choosing defaults.
- Include adversarial text that attempts to instruct the inspection model.

## First release scope

The smallest useful release consists of milestones 1–5 with CLI-only review. Exclude `--deep`, UI support, document-wide second-pass analysis, model-generated confidence, and approximate matching until the reviewable path has demonstrated acceptable recall and masking burden.
