# System Architecture

## High-Level Overview

```
┌─────────────────────────────────────────────────────────┐
│                    Your Machine                          │
│                                                          │
│  ┌───────────┐    Matrix C-S API    ┌────────────────┐  │
│  │           │ ◄──────────────────► │                │  │
│  │  Zap TUI  │    (localhost:6167)  │  Conduit       │  │
│  │  (Rust)   │                      │  (homeserver)  │  │
│  └───────────┘                      └───────┬────────┘  │
│                                             │            │
│                                    Appservice API        │
│                                             │            │
│                                     ┌───────┴────────┐  │
│                                     │  mautrix-meta  │  │
│                                     │  (bridge)      │  │
│                                     └───────┬────────┘  │
│                                             │            │
└─────────────────────────────────────────────┼────────────┘
                                              │ HTTPS
                                     ┌────────┴────────┐
                                     │  Meta/Facebook  │
                                     │  Servers        │
                                     └─────────────────┘
```

## Components

1. **Conduit** — Lightweight Rust Matrix homeserver. Single binary, SQLite, runs on localhost. ~5MB RAM.
2. **mautrix-meta** — Bridges Meta Messenger into Matrix. Logs into your Meta account, creates Matrix rooms for each conversation. Actively maintained (Go).
3. **Zap** — TUI client. Talks only to Conduit via the standard Matrix Client-Server API. Never touches Meta directly.

## Why Matrix bridges?

- Zap only needs to implement one protocol (Matrix), regardless of how many platforms you add later
- Adding Telegram = add `mautrix-telegram` bridge. Adding Signal = add `mautrix-signal`. Zap code doesn't change.
- Conduit + bridges can stay running as background services while Zap launches/exits freely from tmux

## Application Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         Zap Binary                           │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │                    Event Loop (tokio)                  │   │
│  │                                                        │   │
│  │   tokio::select! {                                     │   │
│  │     key_event    = terminal.next()  => dispatch(),     │   │
│  │     matrix_event = sync_stream.next() => update(),     │   │
│  │     tick          = interval.tick() => redraw(),       │   │
│  │   }                                                    │   │
│  └──────────┬──────────────────┬──────────────────┬──────┘   │
│             │                  │                  │           │
│  ┌──────────▼──────┐ ┌────────▼────────┐ ┌──────▼───────┐  │
│  │   Input Layer   │ │  Matrix Layer   │ │  UI Layer    │  │
│  │                 │ │                 │ │              │  │
│  │ - Keymap config │ │ - matrix-sdk    │ │ - Ratatui    │  │
│  │ - Mode handling │ │ - Sync loop     │ │ - Widgets    │  │
│  │ - Command parse │ │ - Room mgmt     │ │ - Theme      │  │
│  └────────┬────────┘ │ - Message send  │ │ - Layout     │  │
│           │          │ - E2EE          │ └──────┬───────┘  │
│           │          └────────┬────────┘        │           │
│           │                   │                  │           │
│  ┌────────▼───────────────────▼──────────────────▼───────┐  │
│  │                    App State                           │  │
│  │                                                        │  │
│  │  - rooms: Vec<Room>        - selected_room: usize     │  │
│  │  - messages: BTreeMap      - input_buffer: String     │  │
│  │  - mode: Mode (Normal/Insert/Command)                 │  │
│  │  - config: Config          - theme: Theme             │  │
│  └──────────────────────┬────────────────────────────────┘  │
│                         │                                    │
│  ┌──────────────────────▼────────────────────────────────┐  │
│  │                  Storage Layer                         │  │
│  │                                                        │  │
│  │  - matrix-sdk SQLite store (state, crypto, sync)      │  │
│  │  - rusqlite (drafts, local prefs, search index)       │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Module Breakdown

| Module | Responsibility |
|--------|---------------|
| `main.rs` | CLI args (clap), init logging, launch tokio runtime |
| `app.rs` | App state struct, main event loop, mode transitions |
| `matrix/` | Matrix client init, sync handling, room/message operations |
| `ui/` | Ratatui layout, widgets (room list, messages, compose bar, status bar) |
| `input/` | Keymap loading, key event to action mapping, mode-aware dispatch |
| `config/` | TOML parsing: `config.toml`, `keymap.toml`, `theme.toml` |
| `store/` | Local SQLite for drafts, preferences, search cache |

## Modes (vim-style)

| Mode | Purpose | Enter | Exit |
|------|---------|-------|------|
| **Normal** | Navigate rooms/messages | `Esc` | — |
| **Insert** | Type a message | `i` | `Esc` |
| **Command** | `:` commands (`:quit`, `:search`, `:rooms`) | `:` | `Esc` / `Enter` |
