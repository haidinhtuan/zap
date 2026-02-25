# Technology Stack

## Dependencies

| Category | Library | Version | Purpose |
|----------|---------|---------|---------|
| **TUI** | ratatui | 0.30 | UI framework |
| | crossterm | 0.28 | Terminal backend (event-stream feature) |
| | tui-textarea | 0.7 | Compose bar widget |
| | ratatui-image | 8.0 | Image rendering (post-MVP) |
| **Matrix** | matrix-sdk | 0.14 | Client SDK (e2e-encryption, bundled-sqlite, sso-login) |
| **Async** | tokio | 1.x | Runtime (rt-multi-thread, macros, sync, time) |
| | futures | 0.3 | Stream combinators |
| **Config** | serde | 1.x | Serialization |
| | toml | 0.8 | Config file parsing |
| | clap | 4.x | CLI arguments |
| | dirs | 5.x | XDG paths |
| **Logging** | tracing | 0.1 | Structured logging |
| | tracing-subscriber | 0.3 | Log subscriber |
| | tracing-appender | 0.2 | File appender |
| **Error** | color-eyre | 0.6 | Error reports + terminal restore on panic |
| | thiserror | 1.x | Custom error types |
| **Unicode** | unicode-width | 0.1 | Display width calculation (CJK, Vietnamese diacritics) |
| | unicode-segmentation | 1.x | Grapheme clusters (combining characters, diacritics) |
| | unicode-normalization | 0.1 | NFC/NFD normalization (critical for Vietnamese text) |
| | emojis | 0.5 | Emoji shortcodes |
| **Storage** | rusqlite | 0.38 | Local data (bundled SQLite) |
| **Clipboard** | arboard | 3.x | Copy/paste (wayland-data-control) |
| **Utilities** | chrono | 0.4 | Timestamps |
| | url | 2.x | URL parsing |
| | linkify | 0.10 | URL detection in messages |
| | rpassword | 7.x | Password input |
| | open | 5.x | Open URLs in browser |

## Build Requirements

- Rust 1.80+ (2024 edition)
- No system dependencies — SQLite is bundled, crossterm is pure Rust

## Infrastructure Requirements

| Component | Install method | Runs as |
|-----------|---------------|---------|
| **Conduit** | Download binary or `cargo install` | systemd service |
| **mautrix-meta** | Download binary or build from Go source | systemd service |
