# Contributing

Thank you for helping improve Decretum.

## Setup

1. Install stable Rust.
2. Clone the repo.
3. Run:

```bash
cargo test
```

## Development guidelines

- Keep changes scoped and explain tradeoffs in PR descriptions.
- Prefer existing project patterns over introducing new abstractions.
- Add or update tests when behavior changes.
- Keep generated artifacts out of version control (usually: `build/`, `target/`).

## Before opening a PR

Run:

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test --all-targets
```
(Some are perceived to be within acceptable range if they do not fundamentally break Decretum, or add unnecessary redundance.)