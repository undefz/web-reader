use anyhow::Result;
use serde::Deserialize;

use crate::post::{Post, Kind};

const TOP_STORIES_URL: &str = "https://hacker-news.firebaseio.com/v0/topstories.json";
const ITEM_URL: &str = "https://hacker-news.firebaseio.com/v0/item";

#[derive(Deserialize)]
struct HnItem {
    id: u64,
    title: Option<String>,
    url: Option<String>,
    text: Option<String>,
    score: Option<i32>,
    time: Option<i64>,
    #[serde(rename = "type")]
    item_type: Option<String>,
}

pub async fn fetch_top_stories(
    limit: usize,
    seen: &std::collections::HashSet<String>,
) -> Result<Vec<Post>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("web/0.1")
        .build()?;

    let ids: Vec<u64> = client
        .get(TOP_STORIES_URL)
        .send()
        .await?
        .json()
        .await?;

    let unseen_ids: Vec<u64> = ids
        .into_iter()
        .take(limit)
        .filter(|id| !seen.contains(&format!("hn:{id}")))
        .collect();

    let mut tasks = Vec::new();
    for id in unseen_ids {
        let client = client.clone();
        tasks.push(tokio::spawn(async move {
            let url = format!("{ITEM_URL}/{id}.json");
            client.get(&url).send().await?.json::<HnItem>().await.map_err(anyhow::Error::from)
        }));
    }

    let mut posts = Vec::new();
    for task in tasks {
        match task.await {
            Ok(Ok(item)) => {
                if item.item_type.as_deref() != Some("story") {
                    continue;
                }
                let title = item.title.unwrap_or_default();
                let body = item
                    .text
                    .as_deref()
                    .map(|t| html_to_text(t))
                    .unwrap_or_default();
                let text = if !body.is_empty() {
                    format!("{title}\n\n{body}")
                } else {
                    title
                };
                let date = chrono::DateTime::from_timestamp(item.time.unwrap_or(0), 0)
                    .unwrap_or_else(chrono::Utc::now);

                posts.push(Post {
                    source: format!("HN ({})", item.score.unwrap_or(0)),
                    text,
                    date,
                    view_count: item.score,
                    id: Some(format!("hn:{}", item.id)),
                    link: item.url.or(Some(format!("https://news.ycombinator.com/item?id={}", item.id))),
                    kind: Kind::HackerNews,
                });
            }
            _ => continue,
        }
    }

    posts.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(posts)
}

fn html_to_text(html: &str) -> String {
    if html.contains('<') {
        html2text::from_read(html.as_bytes(), 80).unwrap_or_else(|_| html.to_string())
    } else {
        html.to_string()
    }
}
