# Contributing

## Build
```sh
cargo build --release
```

## Verify
```sh
cargo build --release && cargo clippy --release -- -D warnings && cargo fmt --check
```

All three must pass before any pull request is considered.

## Style
- `cargo fmt` before committing.
- `cargo clippy` must be clean with `-D warnings`.
- No new dependencies without a one-line justification.
- No tests/CI beyond the existing verification command (Ponytail philosophy:
  the shortest working diff wins; no scaffolding for later).
- Every `unsafe` block needs a `// SAFETY:` comment and lives in `src/term.rs`.
- Every deliberate shortcut gets a `// ponytail: <ceiling>. <upgrade path>.` comment.
- Silence on success: no `println!` in runtime output (errors go to stderr).

## License
By contributing, you agree that your contributions will be licensed under GPL-3.0-only.
Keep the `// SPDX-License-Identifier: GPL-3.0-only` header on every `src/*.rs` file.
