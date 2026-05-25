use anyhow::Result;
use grammers_client::Client;
use std::collections::HashMap;

use crate::config::Config;
use crate::post::{Post, Kind};
use crate::state::State;
use crate::{hn, rss, telegram};

pub async fn collect_posts(
    client: &Client,
    config: &Config,
    state: &mut State,
) -> Result<Vec<Post>> {
    let (tg_result, rss_result, hn_result) = tokio::join!(
        telegram::fetch_unread_posts(client, &config.filter),
        rss::fetch_feeds(&config.rss),
        async {
            if config.hacker_news.enabled {
                hn::fetch_top_stories(config.hacker_news.limit, &state.seen).await
            } else {
                Ok(vec![])
            }
        },
    );

    let mut posts = tg_result?;

    match rss_result {
        Ok(rss_posts) => posts.extend(rss_posts),
        Err(e) => eprintln!("Warning: RSS fetch failed: {e}"),
    }
    match hn_result {
        Ok(hn_posts) => posts.extend(hn_posts),
        Err(e) => eprintln!("Warning: HN fetch failed: {e}"),
    }

    posts.retain(|p| match &p.id {
        Some(id) => !state.is_seen(id),
        None => true,
    });

    sort_for_display(&mut posts);

    for post in &posts {
        if let Some(id) = &post.id {
            state.mark_seen(id.clone());
        }
    }

    Ok(posts)
}

// Order: by source type (HN, RSS, TG), then groups (type, source) by their
// freshest post, then posts within a group by date desc.
fn sort_for_display(posts: &mut [Post]) {
    let mut group_newest: HashMap<(Kind, String), chrono::DateTime<chrono::Utc>> = HashMap::new();
    for p in posts.iter() {
        let key = (p.kind, p.source.clone());
        let entry = group_newest.entry(key).or_insert(p.date);
        if p.date > *entry {
            *entry = p.date;
        }
    }
    posts.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then_with(|| {
                let a_newest = group_newest[&(a.kind, a.source.clone())];
                let b_newest = group_newest[&(b.kind, b.source.clone())];
                b_newest.cmp(&a_newest)
            })
            .then_with(|| b.date.cmp(&a.date))
    });
}
