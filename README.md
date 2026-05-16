# web

A terminal feed reader that combines Telegram channels, RSS feeds, and Hacker News into a single unified view. Built with Rust and Ratatui.

## Install

```sh
cargo build --release
cp target/release/web ~/.local/bin/  # or wherever you keep binaries
```

## Configuration

Create `~/.config/web.json`:

```json
{
  "telegram": {
    "api_id": 12345,
    "api_hash": "your_api_hash",
    "session_file": "~/.config/web.session",
    "phone": "+1234567890"
  },
  "rss": [
    "https://blog.example.com/feed.xml"
  ],
  "hacker_news": {
    "enabled": true,
    "limit": 30
  },
  "state_file": "~/.config/web.state.json",
  "filter": {
    "min_negative_reactions": 10,
    "negative_emojis": ["🤡", "💩"]
  }
}
```

### Telegram

Get `api_id` and `api_hash` from https://my.telegram.org. The `phone` field is only needed for first-time authentication — after that the session file is reused. The app reads unread messages from broadcast channels only (not groups), filters out posts with excessive negative reactions, and marks them as read.

### RSS

List of RSS/Atom/JSON Feed URLs. HTML content is automatically converted to plain text.

### Hacker News

When enabled, fetches the top N stories from the HN API. Channel name shows the score (e.g. `HN (142)`).

### State

The state file tracks which RSS and HN items have been seen so they don't reappear on the next run. Telegram uses its own read-tracking via the API.

## Usage

```sh
web                        # uses ~/.config/web.json
web /path/to/config.json   # custom config path
```

## Keybindings

| Key | Action |
|-----|--------|
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Space` / `Enter` | Open post detail |
| `Esc` / `Space` | Close detail / go back |
| `v` | Open link in browser |
| `?` | Toggle help |
| `q` | Quit |
