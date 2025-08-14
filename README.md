# Hack Club Slack Emoji Downloader

CLI tool for fetching and downloading all emojis currently in Hack Club's Slack workspace (source: https://badger.hackclub.dev/api/emoji).

Built in Rust.

**⚠️ Note: Download performance is bottlenecked by your network speeds.**

This is my first Rust project. Roast my poor programming conventions all you want <3.

## Usage

`hackclub-slack-emoji-dl [OPTIONS]`

### Options

- `--output-dir <PATH>`: Directory to save emojis (default: "./output")
- `--concurrent <NUMBER>`: Maximum concurrent downloads (default: 100)
- `--api-url <URL>`: Custom API endpoint (default: "https://badger.hackclub.dev/api/emoji")
- `-h, --help`: Display help information

### Building

`cargo build --release`