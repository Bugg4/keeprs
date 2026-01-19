# Keeprs

A modern, fast, and native KeePass client for Linux, built with **Rust**, **GTK4**, and **Relm4**.

![Keeprs Screenshot](screenshot.png)

## Features

- **KeePass Support**: Full support for `.kdbx` databases (KeePass 4.x).
- **Floating Search Palette**: VSCode-style fuzzy search (`Ctrl+P`) for lightning-fast navigation.
- **Miller Columns**: Navigate your folder hierarchy effortlessly with a column-based view.

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
search = "<Control>p"
```

## License

MIT
