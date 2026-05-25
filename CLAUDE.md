# Implementation Notes

## Architecture

~1000 lines of Rust across 11 modules. The app has a two-phase startup: Telegram authentication happens before the TUI (auth needs cooked stdin, TUI uses raw mode), then all data sources are fetched in parallel via `tokio::join!`.

## Modules

- **main.rs** — Entry point. Loads config, handles cooldown, connects to Telegram, enters TUI, calls the pipeline, runs the event loop.
- **config.rs** — Deserializes `~/.config/web.json` via serde. Handles `~` expansion for paths. All fields have serde defaults so only `telegram` is required.
- **post.rs** — Defines `Post` (the single unified post struct used everywhere) and `Kind` (HackerNews/Rss/Telegram). `Post.source` is the human-readable label (channel title / feed name / "HN (score)"); `Post.kind` is the discriminator.
- **pipeline.rs** — Orchestrates the fetch → filter-seen → sort → mark-seen flow. Returns a `Vec<Post>` ready for the UI. Owns the multi-key sort (by type, then group freshness, then post date).
- **telegram.rs** — Uses `grammers-client` (MTProto, not Bot API) to connect as a user account. Iterates dialogs, filters to `Chat::Channel` (broadcast only — grammers guarantees this variant excludes megagroups). Fetches unread messages, filters by negative reactions via raw TL types, marks channels as read. Substitutes `"(media)"` for empty captions after album merging.
- **rss.rs** — Fetches RSS/Atom feeds with `reqwest`, parses with `feed-rs`. Prefers `content` (maps to `content:encoded`) over `summary` for full articles. Converts HTML to plain text with `html2text`.
- **hn.rs** — Fetches HN top story IDs, pre-filters already-seen IDs, then fetches item details in parallel via `tokio::spawn`. Only includes `type: "story"`.
- **state.rs** — JSON-backed `HashSet<String>` for seen item IDs. RSS items use `rss:{entry_id}`, HN uses `hn:{item_id}`. Telegram doesn't need state — it has native read tracking.
- **app.rs** — State machine with 4 screens: Loading, Main, Detail, Help. Owns the post list, selection index, scroll offsets.
- **ui.rs** — Ratatui rendering. Main screen is a one-liner-per-post list (preview computed inline per frame). Detail and Help are centered modals rendered on top of the main list. Uses `Clear` widget before drawing modals.
- **theme.rs** — Color constants. Dark background (`#0c0c14`), amber accents, dim status text.

## Key dependencies

- `grammers-client` 0.7 — Telegram MTProto client. TL types are build-time generated; raw types accessed for dialog unread counts and message reactions.
- `ratatui` 0.29 + `crossterm` 0.28 — TUI framework.
- `feed-rs` 2.3 — Unified RSS/Atom/JSON Feed parser.
- `reqwest` 0.12 with `native-tls` — HTTP client. Uses native-tls (not rustls) because some servers (Medium/Netflix) reject rustls connections.
- `html2text` 0.17 — HTML to plain text conversion for RSS content.

## Data flow

```
Config -> parallel fetch [TG, RSS, HN] -> Vec<FetchedPost>
  -> filter out seen (state file) -> sort by date desc
  -> convert to Vec<ChannelPost> (pre-compute one-liner previews)
  -> mark all as seen, save state -> enter TUI event loop
```

## Gotchas

- Telegram auth must happen before `ratatui::init()` — raw mode captures stdin.
- `grammers-client` `Chat::Channel` is guaranteed broadcast-only (checked via `channel.broadcast` flag in `from_raw`). Megagroups become `Chat::Group`.
- Dialog unread count is on `dialog.raw` (`tl::enums::Dialog::Dialog(d) => d.unread_count`), not exposed as a method.
- Reaction data is on `msg.raw.reactions` — requires traversing TL enum wrappers: `MessageReactions::Reactions` -> `ReactionCount::Count` -> `Reaction::Emoji`.
- `reqwest` needs `user_agent("web/0.1")` — some servers return empty/error without a User-Agent.
- The `open` command (macOS) is used for `v` hotkey — would need `xdg-open` on Linux.
