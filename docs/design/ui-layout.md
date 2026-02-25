# UI Layout & Widgets

## Layout

```
┌──────────────────────────────────────────────────────────┐
│ ⚡ Zap                              Normal  ◆ Connected │  ← Status bar
├──────────────┬───────────────────────────────────────────┤
│ ▌John Doe  2 │ John Doe                     Feb 25 10:32│  ← Room header
│  Alice     ● │───────────────────────────────────────────│
│  Bob         │ John    Hey, are you coming tonight?      │
│  Work Chat 5 │         10:30                             │
│              │                                           │
│  Dev Team    │ You     Yeah, I'll be there at 8          │
│              │         10:31                             │
│              │                                           │
│              │ John    Perfect, see you then! 👍          │
│              │         10:32                             │
│              │                                           │
│              │                                           │
│              │                                           │
├──────────────┤───────────────────────────────────────────│
│ /filter...   │ > █                                       │  ← Compose bar
├──────────────┴───────────────────────────────────────────┤
│ [i]nsert [/]search [:]command           j/k:nav ⚡zap   │  ← Help bar
└──────────────────────────────────────────────────────────┘
```

## Widgets

| Widget | Details |
|--------|---------|
| **Status bar** (top) | App name, current mode indicator, connection status dot |
| **Room list** (left panel) | Room name, unread count badge, typing indicator (●), highlight selected |
| **Room header** | Current room name, last activity timestamp |
| **Message view** | Sender name, message body, timestamp. Your messages visually distinct (different color) |
| **Room filter** (bottom-left) | `/` activates, fuzzy-filter room list as you type |
| **Compose bar** | tui-textarea widget, multi-line with `Shift+Enter` |
| **Help bar** (bottom) | Context-sensitive hints, changes with current mode |

## Visual Style

- Color scheme loaded from `theme.toml` — ship with a dark default (Catppuccin Mocha-inspired)
- Unread rooms bolded with count badge
- Your messages in accent color, others in default foreground
- Timestamps dimmed/muted
- Unicode box-drawing for borders
- Minimal chrome — maximize message space for tmux popup use
