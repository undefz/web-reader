use crate::post::Post;

pub enum Screen {
    Loading(String),
    Main,
    Detail(usize),
    Help,
}

pub struct App {
    pub screen: Screen,
    pub posts: Vec<Post>,
    pub selected: usize,
    pub scroll_offset: usize,
    pub list_offset: usize,
    pub should_quit: bool,
}

impl App {
    pub fn new(posts: Vec<Post>) -> Self {
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
