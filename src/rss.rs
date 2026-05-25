use anyhow::Result;

use crate::post::{Kind, Post};

fn html_to_text(html: &str) -> String {
    if html.contains('<') {
        html2text::from_read(html.as_bytes(), 80).unwrap_or_else(|_| html.to_string())
    } else {
        html.to_string()
    }
}

pub async fn fetch_feeds(urls: &[String]) -> Result<Vec<Post>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("web/0.1")
        .build()?;

    let mut posts = Vec::new();

    for url in urls {
        match fetch_one(&client, url).await {
            Ok(mut feed_posts) => posts.append(&mut feed_posts),
            Err(e) => eprintln!("Warning: failed to fetch RSS {url}: {e}"),
        }
    }

    Ok(posts)
}

async fn fetch_one(client: &reqwest::Client, url: &str) -> Result<Vec<Post>> {
    let body = client.get(url).send().await?.bytes().await?;
    let feed = feed_rs::parser::parse(body.as_ref())?;

    let feed_title = feed
        .title
        .map(|t| t.content)
        .unwrap_or_else(|| url.to_string());

    let mut posts = Vec::new();
    for entry in feed.entries {
        let id = if entry.id.is_empty() {
            entry
                .links
                .first()
                .map(|l| l.href.clone())
                .unwrap_or_default()
        } else {
            entry.id
        };

        if id.is_empty() {
            continue;
        }

        let title = entry
            .title
            .map(|t| html_to_text(&t.content).trim().to_string())
            .unwrap_or_default();

        let body_text = entry
            .content
            .and_then(|c| c.body)
            .or_else(|| entry.summary.map(|t| t.content))
            .map(|s| html_to_text(&s))
            .unwrap_or_default();

        let text = if !title.is_empty() && !body_text.is_empty() {
            format!("{title}\n\n{body_text}")
        } else if !title.is_empty() {
            title
        } else {
            body_text
        };

        let date = entry
            .published
            .or(entry.updated)
            .unwrap_or_else(chrono::Utc::now);

        let link = entry.links.first().map(|l| l.href.clone());

        posts.push(Post {
            source: feed_title.clone(),
            text,
            date,
            view_count: None,
            id: Some(format!("rss:{id}")),
            link,
            kind: Kind::Rss,
        });
    }

    Ok(posts)
}
