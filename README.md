# Mask

Mask is a local privacy tool for working with sensitive documents and LLMs. It replaces personally identifiable information with reversible tokens, stores the restore map in a local vault, and restores the original text when processing is complete.

Mask includes a CLI and web UI for encoding and decoding text. It can also extract text from PDFs and use local models to inspect documents for sensitive context such as occupations.

# Demo

[![Mask demo](https://img.youtube.com/vi/LAa5t7cB9Jc/maxresdefault.jpg)](https://youtu.be/LAa5t7cB9Jc)

# Setup

Build the Rust CLI:

```sh
cargo build
```

Install the web UI dependencies:

```sh
cd ui
bun install
```

# Commands

Show all available commands:

```sh
cargo run -- --help
```

Encode a text file and save the restore map locally:

```sh
cargo run -- encode --input input.txt --output data/encoded.txt
```

Decode it again:

```sh
cargo run -- decode --input data/encoded.txt --output data/decoded.txt
```

Start the web app at `http://localhost:8000`:

```sh
cd ui
bun run build
cd ..
cargo run -- serve
```

Mask stores generated files and the default vault snapshot in `data/`.

# Tests

Run the Rust tests:

```sh
cargo test
```

Run the UI tests:

```sh
cd ui
bunx playwright install chromium
bun run test:ui
```
