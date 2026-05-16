use ratatui::prelude::*;
use ratatui::widgets::*;

use crate::app::{App, Screen};
use crate::theme;

pub fn render(frame: &mut Frame, app: &App) {
    match &app.screen {
        Screen::Loading(msg) => render_loading(frame, msg),
        Screen::Main => render_main(frame, app),
        Screen::Detail(idx) => render_detail(frame, app, *idx),
        Screen::Help => {
            render_main(frame, app);
            render_help(frame);
        }
    }
}

fn render_loading(frame: &mut Frame, message: &str) {
    let area = frame.area();
    frame.render_widget(Block::default().style(theme::base()), area);

    let y = area.height / 2;
    let centered = Rect::new(0, y, area.width, 1);
    let text = Paragraph::new(message)
        .alignment(Alignment::Center)
        .style(theme::base());
    frame.render_widget(text, centered);
}

fn render_main(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(area);

    // Title bar
    let title = Line::from(vec![
        Span::styled(" TG Reader ", theme::channel_name()),
        Span::styled(
            format!(" {} posts", app.posts.len()),
            theme::dim(),
        ),
    ]);
    frame.render_widget(
        Paragraph::new(title).style(theme::status_bar()),
        chunks[0],
    );

    // Post list
    let visible_height = chunks[1].height as usize;
    let visible_posts = &app.posts[app.list_offset..];

    for (i, post) in visible_posts.iter().enumerate().take(visible_height) {
        let global_idx = app.list_offset + i;
        let is_selected = global_idx == app.selected;

        let style = if is_selected {
            theme::selected()
        } else {
            theme::base()
        };

        let chan_style = if is_selected {
            theme::channel_name_selected()
        } else {
            theme::channel_name()
        };

        let line = Line::from(vec![
            Span::styled(format!("[{}] ", post.channel_name), chan_style),
            Span::styled(&post.preview, style),
        ]);

        let y = chunks[1].y + i as u16;
        let row_area = Rect::new(chunks[1].x, y, chunks[1].width, 1);
        frame.render_widget(Paragraph::new(line).style(style), row_area);
    }

    // Fill remaining rows with background
    let rendered = visible_posts.len().min(visible_height);
    for i in rendered..visible_height {
        let y = chunks[1].y + i as u16;
        let row_area = Rect::new(chunks[1].x, y, chunks[1].width, 1);
        frame.render_widget(Block::default().style(theme::base()), row_area);
    }

    // Status bar
    let status = Line::from(vec![
        Span::styled(" j/k", Style::default().fg(theme::FG).bg(theme::STATUS_BG)),
        Span::styled(": navigate  ", theme::status_bar()),
        Span::styled("Space", Style::default().fg(theme::FG).bg(theme::STATUS_BG)),
        Span::styled(": open  ", theme::status_bar()),
        Span::styled("q", Style::default().fg(theme::FG).bg(theme::STATUS_BG)),
        Span::styled(": quit  ", theme::status_bar()),
        Span::styled("?", Style::default().fg(theme::FG).bg(theme::STATUS_BG)),
        Span::styled(": help", theme::status_bar()),
    ]);
    frame.render_widget(
        Paragraph::new(status).style(theme::status_bar()),
        chunks[2],
    );
}

fn render_detail(frame: &mut Frame, app: &App, post_idx: usize) {
    let post = &app.posts[post_idx];
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(area);

    // Header
    let mut header_lines = vec![
        Line::from(Span::styled(
            format!(" {}", post.channel_name),
            theme::channel_name(),
        )),
        Line::from(Span::styled(
            format!(" {}", post.date.format("%Y-%m-%d %H:%M UTC")),
            theme::dim(),
        )),
    ];
    if let Some(views) = post.view_count {
        header_lines[1] = Line::from(vec![
            Span::styled(
                format!(" {}", post.date.format("%Y-%m-%d %H:%M UTC")),
                theme::dim(),
            ),
            Span::styled(format!("  |  {} views", views), theme::dim()),
        ]);
    }
    let header = Paragraph::new(header_lines).style(theme::base()).block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(theme::BORDER).bg(theme::BG)),
    );
    frame.render_widget(header, chunks[0]);

    // Body
    let body = Paragraph::new(post.text.as_str())
        .wrap(Wrap { trim: false })
        .scroll((app.scroll_offset as u16, 0))
        .style(theme::base());
    frame.render_widget(body, chunks[1]);

    // Status bar
    let status = Line::from(vec![
        Span::styled(" Esc/Space", Style::default().fg(theme::FG).bg(theme::STATUS_BG)),
        Span::styled(": back  ", theme::status_bar()),
        Span::styled("j/k", Style::default().fg(theme::FG).bg(theme::STATUS_BG)),
        Span::styled(": scroll", theme::status_bar()),
    ]);
    frame.render_widget(
        Paragraph::new(status).style(theme::status_bar()),
        chunks[2],
    );
}

fn render_help(frame: &mut Frame) {
    let area = frame.area();

    let popup_width = (area.width * 60 / 100).max(40).min(60);
    let popup_height = 12_u16.min(area.height);
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let help_text = vec![
        Line::from(""),
        Line::from(Span::styled("  Keybindings", theme::channel_name())),
        Line::from(""),
        Line::from(Span::styled("  j / ↓       Move down", theme::base())),
        Line::from(Span::styled("  k / ↑       Move up", theme::base())),
        Line::from(Span::styled("  Space       Open post / Go back", theme::base())),
        Line::from(Span::styled("  Enter       Open post", theme::base())),
        Line::from(Span::styled("  Esc         Go back", theme::base())),
        Line::from(Span::styled("  ?           Toggle this help", theme::base())),
        Line::from(Span::styled("  q           Quit", theme::base())),
        Line::from(""),
    ];

    let help = Paragraph::new(help_text).style(theme::base()).block(
        Block::default()
            .title(" Help ")
            .title_style(theme::channel_name())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::BORDER))
            .style(theme::base()),
    );

    frame.render_widget(help, popup_area);
}
