# Releases

Pre-built binaries for each release are available below. See [installation instructions](#installation) for details.

## Installation

### Download a binary

1. Download the archive for your platform from the latest release below
2. Extract it:
   ```bash
   tar xzf starkbot-v*-linux-x86_64.tar.gz   # Linux/macOS
   unzip starkbot-v*-windows-x86_64.zip       # Windows
   ```
3. Move the binary to your PATH:
   ```bash
   sudo mv starkbot-v*/starkbot /usr/local/bin/   # Linux/macOS
   ```

### Build from source

```bash
git clone https://github.com/Starkbotai/starkbot-cli.git
cd starkbot-cli
cargo build --release
# Binary at ./target/release/starkbot
```

### Verify checksums

Each release includes a `checksums-v*.sha256` file. Verify your download:

```bash
sha256sum -c checksums-v0.1.0.sha256
```

---

## v0.1.0

> Initial release

**Date:** 2026-05-06

### Highlights

- Terminal-native AI agent platform with interactive TUI (Chat, Skills, Graph, Memory views)
- 10 built-in agent tools (bash, read/write/edit file, grep, find, web fetch, sub-agent, and more)
- 4 personas: coding-agent, research-agent, director, devops-agent
- Skills system with hot-reloadable markdown definitions
- Interactive approval system for tool execution safety
- SQLite-backed memory with full-text search
- Context management with sliding-window compaction

### Downloads

| Platform | Architecture | Download |
|----------|-------------|----------|
| Linux | x86_64 | `starkbot-v0.1.0-linux-x86_64.tar.gz` |
| Linux | aarch64 | `starkbot-v0.1.0-linux-aarch64.tar.gz` |
| macOS | x86_64 (Intel) | `starkbot-v0.1.0-macos-x86_64.tar.gz` |
| macOS | aarch64 (Apple Silicon) | `starkbot-v0.1.0-macos-aarch64.tar.gz` |
| Windows | x86_64 | `starkbot-v0.1.0-windows-x86_64.zip` |

**Checksums:** `checksums-v0.1.0.sha256`

---

*To add a new release, copy the template above and update the version, date, highlights, and download links.*
