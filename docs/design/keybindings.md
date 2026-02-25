# Keybindings & Navigation

All keybindings are configurable via `keymap.toml`. These are the defaults (vim-style).

## Normal Mode

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate room list up/down |
| `Enter` | Open selected room |
| `i` | Enter Insert mode (focus compose bar) |
| `g` `g` | Jump to first room |
| `G` | Jump to last room |
| `/` | Focus room filter |
| `:` | Enter Command mode |
| `Ctrl+u` / `Ctrl+d` | Scroll messages half-page up/down |
| `Tab` | Toggle focus between room list and message view |
| `r` | Mark current room as read |
| `R` | Mark all rooms as read |
| `q` | Quit |

## Insert Mode

| Key | Action |
|-----|--------|
| `Enter` | Send message |
| `Shift+Enter` | New line |
| `Esc` | Back to Normal mode |
| `Ctrl+p` | Scroll messages up (while composing) |
| `Ctrl+n` | Scroll messages down (while composing) |

## Command Mode

| Command | Action |
|---------|--------|
| `:quit` / `:q` | Quit Zap |
| `:search <text>` | Search messages in current room |
| `:rooms` | List all rooms |
| `:react <emoji>` | React to last message |
| `:open` | Open last URL in browser |
| `:theme <name>` | Switch theme |

## Example `keymap.toml`

```toml
[normal]
"j" = "room_next"
"k" = "room_prev"
"i" = "mode_insert"
"/" = "room_filter"
":" = "mode_command"
"q" = "quit"
"g g" = "room_first"
"G" = "room_last"
"Ctrl+u" = "scroll_up_half"
"Ctrl+d" = "scroll_down_half"

[insert]
"Enter" = "send_message"
"Shift+Enter" = "newline"
"Esc" = "mode_normal"
```
