#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Source {
    HackerNews,
    Rss,
    Telegram,
}

pub struct FetchedPost {
    pub channel_name: String,
    pub text: String,
    pub date: chrono::DateTime<chrono::Utc>,
    pub view_count: Option<i32>,
    pub id: Option<String>,
    pub link: Option<String>,
    pub source: Source,
}
