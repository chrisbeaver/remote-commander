# Remote Commander

A Norton Commander-style dual-pane file manager for the terminal, built in Rust. Navigate local and remote filesystems side-by-side with keyboard-driven controls.

![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- **Dual-pane interface** — Classic Norton Commander layout with local filesystem on the left and remote (SSH) on the right
- **Keyboard-driven navigation** — Arrow keys, Page Up/Down, Home/End for fast file browsing
- **Function key commands** — F1-F10 shortcuts for common operations (view, edit, copy, move, delete)
- **SSH remote browsing** — Connect to remote hosts and browse files seamlessly *(coming soon)*
- **Cross-panel operations** — Copy and move files between local and remote systems *(coming soon)*

## Installation

### From Source

```bash
git clone https://github.com/yourusername/remote-commander.git
cd remote-commander
cargo build --release
```

The binary will be available at `target/release/remote-commander`.

## Usage

```bash
# Launch with local filesystem on both panels
remote-commander

# Connect to a remote host (SSH)
remote-commander user@hostname
```

## Keyboard Shortcuts

### Navigation

| Key | Action |
|-----|--------|
| `↑` / `↓` | Move selection up/down |
| `Page Up` / `Page Down` | Move one page up/down |
| `Home` / `End` | Jump to first/last item |
| `Enter` | Enter directory |
| `Backspace` | Go to parent directory |
| `Tab` | Switch between panels |

### Function Keys

| Key | Action |
|-----|--------|
| `F1` | Help |
| `F2` | Menu |
| `F3` | View file |
| `F4` | Edit file |
| `F5` | Copy |
| `F6` | Move/Rename |
| `F7` | Make directory |
| `F8` | Delete |
| `F9` | Terminal |
| `F10` / `q` | Quit |

## Project Structure

```
src/
├── main.rs        # Entry point, CLI parsing, event loop
├── app.rs         # Application state and command handlers
├── file_panel.rs  # Panel logic (selection, scrolling, navigation)
├── filesystem.rs  # Filesystem abstraction (local/remote)
└── ui.rs          # Terminal UI rendering with Ratatui
```

## Dependencies

- [Ratatui](https://ratatui.rs/) — Terminal UI framework
- [Crossterm](https://github.com/crossterm-rs/crossterm) — Cross-platform terminal manipulation
- [Clap](https://clap.rs/) — Command-line argument parsing
- [Russh](https://github.com/warp-tech/russh) — SSH client library *(for remote support)*

## Development

```bash
# Run tests
cargo test

# Run with debug output
RUST_LOG=debug cargo run

# Build release version
cargo build --release
```

## Roadmap

- [x] Dual-pane local filesystem navigation
- [x] Function key bar
- [x] Help popup
- [ ] SSH remote filesystem browsing
- [ ] File copy/move operations
- [ ] File viewing (F3)
- [ ] File editing with external editor (F4)
- [ ] Directory creation (F7)
- [ ] File/directory deletion (F8)
- [ ] File search
- [ ] Bookmarks

## License

MIT License — see [LICENSE](LICENSE) for details.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request.
