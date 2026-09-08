# CLI

Top level help

cargo run -- --help

# Tests

- Run Rust tests: `cargo test`
- Set up Playwright: `cd ui && bun install && bunx playwright install chromium`
- Run UI tests: `cd ui && bun run test:ui`

# Linters and Formatters

- Format Rust: `cargo fmt && cargo fmt --check`
- Lint Rust: `cargo clippy --all-targets --all-features -- -D warnings`
- Format TypeScript: `cd ui && bun run format`
- Lint TypeScript: `cd ui && bun run lint && bun run typecheck`
- Run the formatter and linter whenever changing the corresponding Rust or TypeScript codebase.

# Rules

- Avoid comments
- Store all data in data folder which is gitignored.
