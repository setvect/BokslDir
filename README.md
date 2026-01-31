# 복슬Dir (Boksl Dir)

A modern dual-panel file manager for the terminal, built with Rust.

## Features

- 🎨 TUI-based dual-panel interface
- 🖥️  Responsive layout (adapts to terminal size)
- 🎨 Color theme support
- ⌨️  Customizable keybindings
- 📁 File operations (copy, move, delete)
- 🔍 File search and filtering
- 📚 Tabs and bookmarks
- 🚀 Fast and memory-efficient

## Project Status

🚧 **Currently in Phase 0: Project Initialization**

This is a work in progress. Currently implemented:
- ✅ Project structure
- ✅ Basic TUI framework
- ✅ Hello World dual-panel UI

## Requirements

- Rust 1.93+ (2021 edition)
- Terminal with Unicode and color support

## Installation

```bash
# Clone the repository
git clone https://github.com/yourusername/boksldir.git
cd boksldir

# Build
cargo build --release

# Run
cargo run
```

## Usage

```bash
# Run in development mode
cargo run

# Build and run release version
cargo build --release
./target/release/boksldir
```

### Keyboard Shortcuts (Current)

- `q` or `Esc` - Quit

## Development

### Project Structure

```
src/
├── main.rs           # Entry point
├── app.rs            # Application state
├── ui/               # UI layer
│   ├── components/   # UI components
│   ├── layout.rs     # Layout system
│   ├── theme.rs      # Theme system
│   └── renderer.rs   # Renderer
├── core/             # Business logic
│   ├── file_manager.rs
│   └── navigator.rs
├── system/           # System layer
│   ├── filesystem.rs
│   └── config.rs
├── models/           # Data models
└── utils/            # Utilities
```

### Documentation

- [Requirements](docs/Requirements.md) - High-level requirements
- [PRD](docs/PRD.md) - Product Requirements Document (detailed features)
- [Architecture](docs/Architecture.md) - System architecture and design

### Build

```bash
# Development build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy
```

## Roadmap

### Phase 0: Project Initialization ✅
- [x] Cargo project setup
- [x] Basic project structure
- [x] Hello World TUI

### Phase 1: UX/UI Foundation (In Progress)
- [ ] Responsive layout system
- [ ] Theme system
- [ ] Basic UI components
- [ ] Event handling

### Phase 2: File System Integration
- [ ] Directory reading
- [ ] File list rendering
- [ ] Navigation

### Phase 3+
- See [PRD.md](docs/PRD.md) for detailed roadmap

## Contributing

This project is currently in early development. Contributions will be welcome once the core functionality is implemented.

## License

MIT

## Credits

Inspired by:
- Total Commander
- Midnight Commander (mc)
- ranger
- broot

Built with:
- [ratatui](https://github.com/ratatui-org/ratatui) - TUI framework
- [crossterm](https://github.com/crossterm-rs/crossterm) - Terminal backend
