# AGENTS.md

## Build
- `cargo build --release` → `./target/release/rmustp`. Single binary, 4 deps
  (`rodio`, `jpeg-decoder`, `png`, `libc`).
- Release profile: `opt-level="z"`, `lto="fat"`, `codegen-units=1`, `strip=true`;
  `panic="abort"` (`ponytail: hook en term.rs restaura también en abort; unwind = +94KB).
- Verifica: `cargo build --release && cargo clippy -- -D warnings && cargo fmt --check`.
  Sin tests/lint/CI (decisión del proyecto); smoke = binario + teclas.

## Structure (Rust rewrite)
- Entry: `src/main.rs` (event loop, keybindings, search/lyrics modes, `-h/-v`).
- `src/audio.rs`: `Player` over `rodio::Sink` (no own threads except one bg meta loader).
- `src/tags.rs`: the only tag scanner (ID3v2/FLAC/Vorbis) + `is_audio_name`/`strip_ext`.
- `src/lyrics.rs`, `src/theme.rs` (9 builtins in `const`, no XML), `src/cover.rs`
  (ANSI half-block only, no Kitty), `src/term.rs` (all `unsafe` + SIGTERM/SIGINT restore),
  `src/ui.rs` (all screens + `browse()`).

## Gotchas
- Themes are builtins in `src/theme.rs`; no `themes/` dir needed at runtime.
- App writes `.last_theme` in CWD (gitignored, like `target/`). Don't commit either.
- POSIX only. Sin TTY (stdin/stdout) → exit 2 limpio. `unsafe` acotado a `src/term.rs`.

## Estado heredado (C++, deprecated)
- Old C++ path (`main.cpp`, `backend/`, `frontend/`, `themes/*.xml`) superseded por la
  reescritura Rust; la fuente de verdad es `src/`. Quedan en el árbol y en git history.
