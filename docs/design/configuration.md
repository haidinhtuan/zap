# Configuration & File Layout

## XDG Directory Structure

```
~/.config/zap/
├── config.toml          # General settings
├── keymap.toml          # Keybindings
└── theme.toml           # Colors and styling

~/.local/share/zap/
├── zap.db               # Zap's local SQLite (drafts, prefs)
└── matrix/              # matrix-sdk SQLite store (state, crypto)

~/.local/share/zap/logs/
└── zap.log              # Tracing log output
```

## Example `config.toml`

```toml
[matrix]
homeserver = "http://localhost:6167"
username = "zap-user"

[ui]
theme = "dark"
room_list_width = 20
timestamp_format = "%H:%M"
show_help_bar = true

[behavior]
vim_mode = true
send_read_receipts = true
```

## Example `theme.toml`

```toml
[colors]
bg = "#1e1e2e"
fg = "#cdd6f4"
accent = "#f9e2af"
my_message = "#a6e3a1"
their_message = "#cdd6f4"
timestamp = "#585b70"
unread_badge = "#f38ba8"
border = "#45475a"
selected_room = "#313244"
status_bar_bg = "#181825"
help_bar_fg = "#585b70"
```

## Infrastructure

Conduit and mautrix-meta run as systemd services. Zap doesn't manage them — it just connects to the homeserver URL in `config.toml`.
