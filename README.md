# Zap

A fast, compact terminal messenger client built in Rust. Designed to run as a tmux popup.

Connects to messaging platforms (starting with Meta Messenger) via Matrix bridges — one protocol to rule them all.

## Features (planned)

- Two-panel TUI: conversation list + message view
- Vim-style keybindings (configurable)
- Themeable via TOML
- End-to-end encryption (via matrix-sdk)
- Persistent sessions
- Fuzzy room filter

## Architecture

```
┌───────────┐    Matrix API    ┌────────────┐    ┌──────────────┐    ┌──────────┐
│  Zap TUI  │ ◄──────────────►│  Conduit   │◄──►│ mautrix-meta │◄──►│ Meta/FB  │
│  (Rust)   │                  │ (homeserver)│    │  (bridge)    │    │ Servers  │
└───────────┘                  └────────────┘    └──────────────┘    └──────────┘
```

Zap talks to a local Matrix homeserver (Conduit). Bridges handle platform integration.
Adding new platforms = adding new bridges. Zap code stays the same.

## Tech Stack

- **TUI:** Ratatui + crossterm
- **Protocol:** matrix-rust-sdk
- **Async:** Tokio
- **Config:** TOML (serde)
- **Storage:** SQLite (rusqlite)

## Building

```bash
cargo build --release
```

## Configuration

```
~/.config/zap/
├── config.toml
├── keymap.toml
└── theme.toml
```

## License

MIT
