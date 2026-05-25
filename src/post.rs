#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    HackerNews,
    Rss,
    Telegram,
}

pub struct Post {
    pub source: String,
    pub text: String,
    pub date: chrono::DateTime<chrono::Utc>,
    pub view_count: Option<i32>,
    pub id: Option<String>,
    pub link: Option<String>,
    pub kind: Kind,
}
