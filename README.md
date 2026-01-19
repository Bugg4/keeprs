# Keeprs

A modern, fast, and native KeePass client for Linux, built with **Rust**, **GTK4**, and **Relm4**.

![Keeprs Screenshot](screenshot.png)

## Features

- **Collapsible Tree Sidebar**: Fully interactive folder tree with visual hierarchy lines, expand/collapse toggles, and auto-sync with search results.
- **Floating Search Palette**: VSCode-style fuzzy search (`Ctrl+P`) for lightning-fast navigation to any entry or group.
- **Entry Management**: Complete view and edit capabilities for entries, including username, password, URL, notes, and custom fields.
- **Attachments Support**: Securely view and download binary attachments stored within your database.
- **TOTP Integration**: Built-in TOTP generator with a visual countdown timer for two-factor authentication codes.

## Installation

### Prerequisites

- Rust (latest stable)
- GTK4 development libraries

On Fedora:
```bash
sudo dnf install gtk4-devel
```

On Ubuntu/Debian:
```bash
sudo apt install libgtk-4-dev
```

### Build & Run

```bash
git clone https://github.com/username/keeprs.git
cd keeprs
cargo run --release
```

## Configuration

Keeprs looks for a config file at `~/.config/keeprs/keeprs.toml`:

```toml
database_path = "/path/to/your/database.kdbx"

[keybindings]
search = "ctrl+p"
```

## License

MIT
