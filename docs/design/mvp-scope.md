# MVP Feature Scope

## MVP (v0.1)

| Feature | Details |
|---------|---------|
| **Login** | Matrix login to local Conduit homeserver (username/password) |
| **Room list** | Display bridged Messenger conversations, sorted by recent activity |
| **Unread indicators** | Bold room name + unread count |
| **Read messages** | Paginated message history with sender, body, timestamp |
| **Send messages** | Plain text messages via compose bar |
| **Room filter** | Fuzzy search/filter room list |
| **Vim keybindings** | Configurable from `keymap.toml` |
| **Theming** | Configurable from `theme.toml`, ship with dark default |
| **E2EE** | Handled by matrix-sdk, transparent to user |
| **Persistent session** | SQLite store — no re-login on restart |

## Post-MVP (v0.2+)

| Feature | Priority |
|---------|----------|
| Reactions (send/display) | High |
| Inline image rendering | High |
| Read receipts (display) | Medium |
| Typing indicators | Medium |
| URL detection + `:open` | Medium |
| Message search | Medium |
| Desktop notifications | Low |
| Markdown rendering | Low |
| File upload/download | Low |
| Additional bridges (Telegram, Signal) | Low |

## Explicitly out of scope

- Group chat management (create/leave/invite)
- Voice/video
- Custom emoji/sticker packs
- Bot interactions
- Multi-account support
