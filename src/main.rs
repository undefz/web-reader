mod app;
mod config;
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
    let (tg_result, rss_result) = tokio::join!(
        telegram::fetch_unread_posts(&client, &config.filter),
        rss::fetch_feeds(&rss_urls),
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

    // Filter out already-seen RSS items
    fetched.retain(|p| match &p.id {
        Some(id) => !state.is_seen(id),
        None => true,
    });

    fetched.sort_by(|a, b| b.date.cmp(&a.date));

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
            KeyCode::Char('?') => app.toggle_help(),
            _ => {}
        },
        Screen::Detail(_) => match key {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Esc | KeyCode::Char(' ') => app.close_detail(),
            KeyCode::Char('j') | KeyCode::Down => app.scroll_down(),
            KeyCode::Char('k') | KeyCode::Up => app.scroll_up(),
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
