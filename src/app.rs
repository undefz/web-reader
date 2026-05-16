use unicode_width::UnicodeWidthStr;

pub enum Screen {
    Loading(String),
    Main,
    Detail(usize),
    Help,
}

pub struct ChannelPost {
    pub channel_name: String,
    pub text: String,
    pub date: chrono::DateTime<chrono::Utc>,
    pub preview: String,
    pub view_count: Option<i32>,
    pub id: Option<String>,
    pub link: Option<String>,
}

pub struct App {
    pub screen: Screen,
    pub posts: Vec<ChannelPost>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub list_offset: usize,
    pub should_quit: bool,
}

impl App {
    pub fn new(posts: Vec<ChannelPost>) -> Self {
        Self {
            screen: Screen::Main,
            posts,
            selected: 0,
            scroll_offset: 0,
            list_offset: 0,
            should_quit: false,
        }
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.list_offset {
                self.list_offset = self.selected;
            }
        }
    }

    pub fn move_down(&mut self, visible_height: usize) {
        if self.selected + 1 < self.posts.len() {
            self.selected += 1;
            if self.selected >= self.list_offset + visible_height {
                self.list_offset = self.selected - visible_height + 1;
            }
        }
    }

    pub fn open_detail(&mut self) {
        if !self.posts.is_empty() {
            self.scroll_offset = 0;
            self.screen = Screen::Detail(self.selected);
        }
    }

    pub fn close_detail(&mut self) {
        self.screen = Screen::Main;
    }

    pub fn toggle_help(&mut self) {
        match self.screen {
            Screen::Help => self.screen = Screen::Main,
            _ => self.screen = Screen::Help,
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll_offset += 1;
    }

    pub fn scroll_up(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(1);
    }
}

pub fn make_preview(channel_name: &str, text: &str, max_width: u16) -> String {
    let prefix = format!("[{}] ", channel_name);
    let prefix_width = UnicodeWidthStr::width(prefix.as_str());
    let available = (max_width as usize).saturating_sub(prefix_width);

    let first_line = text.lines().next().unwrap_or("(no text)");
    let first_line = if first_line.is_empty() {
        "(media)"
    } else {
        first_line
    };

    truncate_to_width(first_line, available)
}

fn truncate_to_width(s: &str, max_width: usize) -> String {
    let mut width = 0;
    let mut result = String::new();
    for ch in s.chars() {
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if width + ch_width > max_width {
            if max_width >= 1 {
                result.push('…');
            }
            break;
        }
        width += ch_width;
        result.push(ch);
    }
    result
}
