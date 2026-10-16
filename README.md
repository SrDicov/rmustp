# rmustp

TUI music player for POSIX systems. Rust rewrite of [cmuspp](https://github.com/Ars-byte/cmuspp).

[![License: GPL-3.0-only](https://img.shields.io/badge/License-GPL--3.0--only-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.65%2B-orange.svg)](https://www.rust-lang.org)

## Features
- Browse, play, pause, seek ±5s, volume
- Shuffle + repeat (loop)
- Metadata: WAV, FLAC, OGG, MP3 (artist/album/title/duration/bitrate)
- Cover art: embedded (FLAC/OGG/MP3 APIC) + directory fallback (`cover.jpg/png`, …)
- Lyrics: `.lrc` synchronized + embedded (MP3/FLAC/OGG)
- 9 built-in themes, switchable at runtime
- POSIX: no daemon, no network, no systemd, no X11

## Requirements
- Linux (verified). POSIX by design, untested on macOS/*BSD.
- ~18 MB RAM typical (~30 MB with `LD_PRELOAD` alternative allocator).
- ALSA headers at build time on Linux (`alsa-lib-devel` or similar).
- Terminal with ANSI true-color + half-block (`▄`) support.

## Install
```sh
cargo build --release
./target/release/rmustp
```

Or download a prebuilt binary from [Releases](https://github.com/SrDicov/rmustp/releases).

## Usage
```sh
rmustp          # file browser starts at $HOME
rmustp -h       # usage
rmustp -v       # version
```

Player keys: `q` quit, `↑/↓` or `k/j` navigate, `Enter` play, `Space` pause,
`←/→` or `h` seek ∓5s, `n`/`p` next/previous, `+`/`-` volume, `s` shuffle,
`r` repeat, `l` lyrics, `t` theme, `/` search, `o` browse, `a` about.
Browser keys: `→`/`Enter`/`l` open, `←`/`Bksp`/`h` up, `o` home, `q` cancel.
Search keys: `↑/↓` select, `Enter` play, `Esc` cancel.

## Build
```sh
cargo build --release
```

## Verification
```sh
cargo build --release && cargo clippy --release -- -D warnings && cargo fmt --check
```

## License
GPL-3.0-only. See [LICENSE](LICENSE).

## Known limitations
- CJK terminal width: `dw()` counts `chars()`, not display cells. Wide chars may misalign.
- `.last_theme` stored in CWD, not `$XDG_STATE_HOME`.
- OGG embedded picture untested (no fixture tooling available).
