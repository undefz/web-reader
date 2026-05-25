mod app;
mod config;
mod hn;
mod pipeline;
mod post;
mod rss;
mod state;
mod telegram;
mod theme;
mod ui;

use anyhow::Result;
use app::{App, Screen};
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

    let pipeline_result = pipeline::collect_posts(&client, &config, &mut state).await;

    if let Err(e) = client.session().save_to_file(&config.telegram.session_file) {
        eprintln!("Warning: failed to save session: {e}");
    }

    let posts = match pipeline_result {
        Ok(p) => p,
        Err(e) => {
            ratatui::restore();
            return Err(e);
        }
    };

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
