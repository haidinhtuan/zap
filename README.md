<!-- Improved compatibility of back to top link: See: https://github.com/othneildrew/Best-README-Template/pull/73 -->
<a id="readme-top"></a>

<!-- PROJECT SHIELDS -->
[![Rust][rust-shield]][rust-url]
[![MIT License][license-shield]][license-url]
[![LinkedIn][linkedin-shield]][linkedin-url]

<!-- PROJECT LOGO -->
<br />
<div align="center">
  <a href="#">
    <img src="assets/screenshot.png" alt="Zap Terminal Messenger" width="100%">
  </a>

  <h3 align="center">Zap</h3>

  <p align="center">
    A fast, compact terminal messenger client built in Rust. Talks to messaging platforms via Matrix bridges — one protocol to rule them all.
    <br />
    <a href="#usage"><strong>Explore the docs »</strong></a>
    <br />
    <br />
    <a href="#features">View Features</a>
    &middot;
    <a href="mailto:me@haidinhtuan.de">Report Bug</a>
    &middot;
    <a href="mailto:me@haidinhtuan.de">Request Feature</a>
  </p>
</div>

<!-- TABLE OF CONTENTS -->
<details>
  <summary>Table of Contents</summary>
  <ol>
    <li>
      <a href="#about-the-project">About The Project</a>
      <ul>
        <li><a href="#features">Features</a></li>
        <li><a href="#built-with">Built With</a></li>
      </ul>
    </li>
    <li>
      <a href="#getting-started">Getting Started</a>
      <ul>
        <li><a href="#prerequisites">Prerequisites</a></li>
        <li><a href="#installation">Installation</a></li>
      </ul>
    </li>
    <li><a href="#usage">Usage</a></li>
    <li><a href="#architecture">Architecture</a></li>
    <li><a href="#configuration">Configuration</a></li>
    <li><a href="#keybindings">Keybindings</a></li>
    <li><a href="#roadmap">Roadmap</a></li>
    <li><a href="#contributing">Contributing</a></li>
    <li><a href="#license">License</a></li>
    <li><a href="#contact">Contact</a></li>
  </ol>
</details>

<!-- ABOUT THE PROJECT -->
## About The Project

![Version](https://img.shields.io/badge/version-0.1.0-blue.svg?style=for-the-badge)

**Zap** is a terminal-based messenger client designed to run as a tmux popup. It connects to Meta Messenger (and other platforms) through Matrix bridges, so you can chat without leaving the terminal.

The idea is simple: a local Matrix homeserver handles the protocol, bridges handle platform integration, and Zap handles the UI. Adding new platforms means adding new bridges — Zap code stays the same.

### Features

*   **Two-Panel TUI** - Room list + message view, compact and keyboard-driven
*   **Vim-Style Navigation** - `j`/`k` for rooms, `Enter` for message select mode, `i` for compose
*   **Vietnamese Input** - Built-in Telex input via [Vigo](https://github.com/haidinhtuan/vigo), toggle with `Ctrl+t`
*   **Reply to Messages** - Select a message with `r`, compose your reply with full context
*   **Delete Messages** - Press `d` on a message, confirm with `y` (Matrix redact API)
*   **Room Sorting** - Rooms sorted by most recent activity, unread indicators
*   **Session Persistence** - Login once, session is saved for future runs
*   **Auto-Join Invites** - Automatically joins rooms you're invited to
*   **End-to-End Encryption** - Via matrix-sdk's E2EE support
*   **Offline Mode** - Start with `--offline` to browse without connecting
*   **Configurable** - Keybindings, themes, and settings via TOML files
*   **Tmux Integration** - Designed to pop up with a single keybinding

<p align="right">(<a href="#readme-top">back to top</a>)</p>

### Built With

**Core Technologies:**
*   [![Rust][rust-shield]][rust-url] - Rust 2021 Edition
*   [![Ratatui][ratatui-shield]][ratatui-url] - Terminal UI framework
*   [![Matrix][matrix-shield]][matrix-url] - matrix-rust-sdk for protocol
*   [![Tokio][tokio-shield]][tokio-url] - Async runtime
*   [![Vigo][vigo-shield]][vigo-url] - Vietnamese input engine

**Infrastructure:**
*   [![Conduit][conduit-shield]][conduit-url] - Lightweight Matrix homeserver
*   [![mautrix][mautrix-shield]][mautrix-url] - Meta Messenger bridge

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- GETTING STARTED -->
## Getting Started

To get a local copy up and running follow these simple steps.

### Prerequisites

*   **Rust** - [Install Rust](https://rustup.rs/)
*   **Matrix homeserver** - A running Conduit (or Synapse) instance
*   **mautrix-meta** - Bridge for Meta Messenger

A Docker Compose file is included to spin up Conduit + mautrix-meta:

```bash
docker compose up -d
```

### Installation

1.  **Clone the repo**
    ```bash
    git clone https://github.com/haidinhtuan/zap.git
    cd zap
    ```

2.  **Build**
    ```bash
    cargo build --release
    ```

3.  **Configure your Matrix credentials**
    ```bash
    # Config files are auto-created on first run at ~/.config/zap/
    # Edit config.toml with your homeserver and username:
    $EDITOR ~/.config/zap/config.toml
    ```

4.  **(Optional) Add to tmux as a popup**
    ```bash
    # Add to ~/.tmux.conf (adjust prefix to your setup):
    bind z display-popup -E -w 85% -h 80% -b rounded -T " ⚡ zap " "/path/to/zap/target/release/zap"
    ```

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- USAGE -->
## Usage

Launch Zap:

```bash
./target/release/zap
```

With options:

```bash
zap --verbose       # Enable debug logging
zap --offline       # Run without Matrix connection
zap --config /path  # Custom config directory
```

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- ARCHITECTURE -->
## Architecture

```
┌───────────┐    Matrix API    ┌────────────┐    ┌──────────────┐    ┌──────────┐
│  Zap TUI  │ ◄──────────────► │  Conduit   │◄──►│ mautrix-meta │◄──►│ Meta/FB  │
│  (Rust)   │                  │ (homeserver)│    │  (bridge)    │    │ Servers  │
└───────────┘                  └────────────┘    └──────────────┘    └──────────┘
```

Zap talks to a local Matrix homeserver. Bridges handle platform integration. Adding new platforms = adding new bridges. Zap code stays the same.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- CONFIGURATION -->
## Configuration

Config files live at `~/.config/zap/` and are auto-created on first run:

```
~/.config/zap/
├── config.toml     # Matrix credentials, UI preferences
├── keymap.toml     # Keybindings for all modes
└── theme.toml      # Color scheme
```

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- KEYBINDINGS -->
## Keybindings

All keybindings are configurable via `keymap.toml`.

| Mode | Key | Action |
| :--- | :--- | :--- |
| **Normal** | `j` / `k` | Navigate rooms |
| | `Enter` | Enter message select mode |
| | `i` | Compose a message |
| | `:` | Command mode |
| | `q` | Quit |
| **Message Select** | `j` / `k` | Navigate messages |
| | `r` | Reply to selected message |
| | `d` | Delete selected message |
| | `Esc` | Back to normal mode |
| **Insert** | `Enter` | Send message |
| | `Ctrl+t` | Toggle Vietnamese input (Telex) |
| | `Ctrl+x` | Cancel compose |
| | `Esc` | Back to normal mode |

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- ROADMAP -->
## Roadmap

- [x] Matrix connection with session persistence
- [x] Room list with unread indicators and activity sorting
- [x] Message display with timestamps and sender colors
- [x] Vim-style message selection and navigation
- [x] Reply to messages
- [x] Delete messages with confirmation
- [x] Nvim-style mode indicator in status bar
- [x] Own message detection and display
- [ ] Message search (`/` in message select mode)
- [ ] Fuzzy room filter
- [ ] Image/file attachment support
- [ ] Custom color themes
- [ ] Multi-account support
- [ ] WhatsApp bridge integration

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- CONTRIBUTING -->
## Contributing

Contributions are what make the open source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- LICENSE -->
## License

Distributed under the MIT License. See `LICENSE` for more information.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- CONTACT -->
## Contact

**Hai Dinh Tuan** - me@haidinhtuan.de

Project Link: [https://github.com/haidinhtuan/zap](https://github.com/haidinhtuan/zap)

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- MARKDOWN LINKS & IMAGES -->
[rust-shield]: https://img.shields.io/badge/Rust-2021-DEA584?style=for-the-badge&logo=rust&logoColor=white
[rust-url]: https://www.rust-lang.org/
[license-shield]: https://img.shields.io/badge/License-MIT-green.svg?style=for-the-badge
[license-url]: LICENSE
[linkedin-shield]: https://img.shields.io/badge/-LinkedIn-black.svg?style=for-the-badge&logo=linkedin&colorB=555
[linkedin-url]: https://linkedin.com/in/haidinhtuan
[ratatui-shield]: https://img.shields.io/badge/Ratatui-Terminal_UI-DEA584?style=for-the-badge&logo=rust&logoColor=white
[ratatui-url]: https://github.com/ratatui/ratatui
[matrix-shield]: https://img.shields.io/badge/Matrix-Protocol-000000?style=for-the-badge&logo=matrix&logoColor=white
[matrix-url]: https://github.com/matrix-org/matrix-rust-sdk
[tokio-shield]: https://img.shields.io/badge/Tokio-Async_Runtime-DEA584?style=for-the-badge&logo=rust&logoColor=white
[tokio-url]: https://tokio.rs/
[conduit-shield]: https://img.shields.io/badge/Conduit-Homeserver-000000?style=for-the-badge&logo=matrix&logoColor=white
[conduit-url]: https://conduit.rs/
[mautrix-shield]: https://img.shields.io/badge/mautrix-Meta_Bridge-0084FF?style=for-the-badge&logo=messenger&logoColor=white
[mautrix-url]: https://github.com/mautrix/meta
[vigo-shield]: https://img.shields.io/badge/Vigo-Vietnamese_Input-DEA584?style=for-the-badge&logo=rust&logoColor=white
[vigo-url]: https://github.com/haidinhtuan/vigo
