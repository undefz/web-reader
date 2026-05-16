mod app;
mod config;
mod hn;
mod rss;
mod state;
mod telegram;
mod theme;
mod ui;

use anyhow::Result;
use app::{make_preview, App, ChannelPost, Screen};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};

#[tokio::main]
async fn main() -> Result<()> {
    let config_path = std::env::args().nth(1).unwrap_or_else(|| {
        let home = std::env::var("HOME").expect("HOME not set");
        format!("{home}/.config/web.json")
    });
    let config = config::Config::load(&config_path)?;

    let mut state = state::State::load(&config.state_file)?;

    if config.cooldown_minutes > 0 {
        if let Some(last_opened) = state.last_opened {
            let elapsed = chrono::Utc::now() - last_opened;
            let cooldown = chrono::Duration::minutes(config.cooldown_minutes as i64);
            if elapsed < cooldown {
                let remaining = cooldown - elapsed;
                let h = remaining.num_hours();
                let m = remaining.num_minutes() % 60;
                let ago = elapsed.num_minutes();
                eprintln!("Cooldown active: {h}h {m}m remaining (opened {ago}m ago, cooldown is {}m)", config.cooldown_minutes);
                std::process::exit(1);
            }
        }
    }

    state.last_opened = Some(chrono::Utc::now());
    state.save(&config.state_file)?;

    println!("Connecting to Telegram...");
    let client = telegram::connect(&config.telegram).await?;

    let mut terminal = ratatui::init();

    terminal.draw(|f| {
        ui::render(
            f,
            &App {
                screen: Screen::Loading("Fetching unread messages...".into()),
                posts: vec![],
                selected: 0,
                scroll_offset: 0,
                list_offset: 0,
                should_quit: false,
            },
        )
    })?;

    let rss_urls = config.rss.clone();
    let hn_config = &config.hacker_news;
    let hn_enabled = hn_config.enabled;
    let hn_limit = hn_config.limit;
    let seen_clone = state.seen.clone();

    let (tg_result, rss_result, hn_result) = tokio::join!(
        telegram::fetch_unread_posts(&client, &config.filter),
        rss::fetch_feeds(&rss_urls),
        async {
            if hn_enabled {
                hn::fetch_top_stories(hn_limit, &seen_clone).await
            } else {
                Ok(vec![])
            }
        },
    );

    if let Err(e) = client.session().save_to_file(&config.telegram.session_file) {
        eprintln!("Warning: failed to save session: {e}");
    }

    let mut fetched = match tg_result {
        Ok(f) => f,
        Err(e) => {
            ratatui::restore();
            return Err(e);
        }
    };

    match rss_result {
        Ok(rss_posts) => fetched.extend(rss_posts),
        Err(e) => eprintln!("Warning: RSS fetch failed: {e}"),
    }

    match hn_result {
        Ok(hn_posts) => fetched.extend(hn_posts),
        Err(e) => eprintln!("Warning: HN fetch failed: {e}"),
    }

    // Filter out already-seen items
    fetched.retain(|p| match &p.id {
        Some(id) => !state.is_seen(id),
        None => true,
    });

    // Sort: by source category (HN, RSS, TG), then by source name
    // (whichever source has the newest post comes first), then newest first within source
    use std::collections::HashMap;
    let mut source_newest: HashMap<(telegram::Source, String), chrono::DateTime<chrono::Utc>> =
        HashMap::new();
    for p in &fetched {
        let key = (p.source, p.channel_name.clone());
        let entry = source_newest.entry(key).or_insert(p.date);
        if p.date > *entry {
            *entry = p.date;
        }
    }
    fetched.sort_by(|a, b| {
        a.source
            .cmp(&b.source)
            .then_with(|| {
                let a_newest = source_newest[&(a.source, a.channel_name.clone())];
                let b_newest = source_newest[&(b.source, b.channel_name.clone())];
                b_newest.cmp(&a_newest)
            })
            .then_with(|| b.date.cmp(&a.date))
    });

    let terminal_width = terminal.size()?.width;
    let posts: Vec<ChannelPost> = fetched
        .into_iter()
        .map(|p| ChannelPost {
            preview: make_preview(&p.channel_name, &p.text, terminal_width),
            channel_name: p.channel_name,
            text: if p.text.is_empty() {
                "(media)".into()
            } else {
                p.text
            },
            date: p.date,
            view_count: p.view_count,
            id: p.id,
            link: p.link,
        })
        .collect();

    // Mark all displayed RSS items as seen
    for post in &posts {
        if let Some(id) = &post.id {
            state.mark_seen(id.clone());
        }
    }
    if let Err(e) = state.save(&config.state_file) {
        eprintln!("Warning: failed to save state: {e}");
    }

    let mut app = App::new(posts);

    loop {
        let visible_height = terminal
            .size()
            .map(|s| s.height.saturating_sub(2) as usize)
            .unwrap_or(20);

        terminal.draw(|frame| ui::render(frame, &app))?;

        if app.should_quit {
            break;
        }

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                handle_key(&mut app, key.code, visible_height);
            }
        }
    }

    ratatui::restore();
    Ok(())
}

fn handle_key(app: &mut App, key: KeyCode, visible_height: usize) {
    match &app.screen {
        Screen::Main => match key {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => app.move_down(visible_height),
            KeyCode::Char('k') | KeyCode::Up => app.move_up(),
            KeyCode::Char(' ') | KeyCode::Enter => app.open_detail(),
            KeyCode::Char('v') => open_selected_link(app),
            KeyCode::Char('?') => app.toggle_help(),
            _ => {}
        },
        Screen::Detail(idx) => match key {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Esc | KeyCode::Char(' ') => app.close_detail(),
            KeyCode::Char('j') | KeyCode::Down => app.scroll_down(),
            KeyCode::Char('k') | KeyCode::Up => app.scroll_up(),
            KeyCode::Char('v') => open_link_at(app, *idx),
            KeyCode::Char('?') => app.toggle_help(),
            _ => {}
        },
        Screen::Help => match key {
            KeyCode::Char('?') | KeyCode::Esc => app.toggle_help(),
            KeyCode::Char('q') => app.should_quit = true,
            _ => {}
        },
        Screen::Loading(_) => {}
    }
}

fn open_selected_link(app: &App) {
    open_link_at(app, app.selected);
}

fn open_link_at(app: &App, idx: usize) {
    if let Some(post) = app.posts.get(idx) {
        if let Some(link) = &post.link {
            let _ = std::process::Command::new("open")
                .arg(link)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
    }
}
