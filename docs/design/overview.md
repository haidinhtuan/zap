# Zap — Overview & Goals

**Zap** is a compact, fast-launching terminal messenger client built in Rust with Ratatui. It connects to messaging platforms (starting with Meta Messenger) via Matrix bridges.

## Goals

- Launch instantly as a tmux popup
- Two-panel UI: conversation list + message view
- Configurable keybindings (vim default)
- Hacker aesthetic — functional with style
- Multi-platform messaging through Matrix bridges
- Full Unicode support (Vietnamese, CJK, emoji, combining characters)

## Non-goals (for now)

- Desktop notifications
- Voice/video calls
- File sharing beyond basic images
- Mobile/web companion

## Target user

Terminal power user in a tmux session, quickly checking and replying to Messenger conversations without leaving the terminal.
