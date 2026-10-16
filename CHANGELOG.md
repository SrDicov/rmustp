# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2026-10-16

### Added
- Initial release. Rust rewrite of cmuspp (C++).
- TUI music player: browse, play, pause, seek, volume, shuffle, repeat.
- Metadata: WAV, FLAC, OGG, MP3 (own minimal ID3v2/FLAC/Vorbis scanner).
- Cover art: embedded (FLAC/OGG/MP3 APIC) + directory fallback, ANSI half-block.
- Lyrics: `.lrc` synchronized (offset, multi-timestamp, decimals, BOM/CRLF) + embedded tags.
- 9 built-in themes, persisted per working directory.
- POSIX-compliant: no daemon, no network, no X11, no systemd; clean exit 2 without TTY.
- Terminal restore on SIGINT/SIGTERM and on panic (hook + `panic="abort"`).
