use anyhow::{bail, Context, Result};
use grammers_client::session::Session;
use grammers_client::types::Chat;
use grammers_client::{Client, Config, InitParams, SignInError};
use grammers_tl_types as tl;
use std::io::{self, Write};

use crate::config::{FilterConfig, TelegramConfig};

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

pub async fn connect(config: &TelegramConfig) -> Result<Client> {
    let client = Client::connect(Config {
        session: Session::load_file_or_create(&config.session_file)?,
        api_id: config.api_id,
        api_hash: config.api_hash.clone(),
        params: InitParams {
            catch_up: false,
            ..Default::default()
        },
    })
    .await
    .context("Failed to connect to Telegram")?;

    if !client.is_authorized().await? {
        let phone = config
            .phone
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Phone number required for first-time auth. Add phone = \"+1234567890\" to config.toml"))?;

        let token = client.request_login_code(phone).await?;

        print!("Enter the login code sent to your Telegram app: ");
        io::stdout().flush()?;
        let mut code = String::new();
        io::stdin().read_line(&mut code)?;
        let code = code.trim();

        match client.sign_in(&token, code).await {
            Ok(_) => println!("Signed in successfully!"),
            Err(SignInError::PasswordRequired(password_token)) => {
                print!(
                    "2FA password required (hint: {:?}): ",
                    password_token.hint()
                );
                io::stdout().flush()?;
                let mut password = String::new();
                io::stdin().read_line(&mut password)?;
                client
                    .check_password(password_token, password.trim())
                    .await?;
                println!("Signed in with 2FA!");
            }
            Err(SignInError::InvalidCode) => bail!("Invalid login code"),
            Err(SignInError::InvalidPassword) => bail!("Invalid 2FA password"),
            Err(SignInError::SignUpRequired { .. }) => bail!("Sign up required — not supported"),
            Err(SignInError::Other(e)) => return Err(e.into()),
        }

        client.session().save_to_file(&config.session_file)?;
    }

    Ok(client)
}

pub async fn fetch_unread_posts(
    client: &Client,
    filter: &FilterConfig,
) -> Result<Vec<FetchedPost>> {
    let mut posts = Vec::new();
    let mut dialogs = client.iter_dialogs();

    while let Some(dialog) = dialogs.next().await? {
        let Chat::Channel(channel) = dialog.chat() else {
            continue;
        };

        let (unread_count, read_inbox_max_id) = match &dialog.raw {
            tl::enums::Dialog::Dialog(d) => (d.unread_count, d.read_inbox_max_id),
            tl::enums::Dialog::Folder(_) => (0, 0),
        };

        if unread_count == 0 {
            continue;
        }

        let channel_name = channel.title().to_string();
        let packed = channel.pack();

        // Stop once we've drained at least unread_count raw messages AND
        // crossed the read boundary. Both bounds are needed: unread_count
        // treats an album as one user-visible item (so capping at it alone
        // cuts off album members), and read_inbox_max_id can run ahead of
        // unread_count when read state syncs from another client (so
        // breaking at it alone misses messages unread_count still considers
        // unread). Count raw iterations rather than pushed posts — the
        // negative-reactions filter and album-member merging both `continue`
        // without growing channel_posts, which would otherwise keep us
        // iterating into already-read messages.
        let mut messages = client.iter_messages(packed);
        let mut raw_msgs: Vec<grammers_client::types::Message> = Vec::new();
        let target = unread_count as usize;
        let mut iter_count = 0;

        while let Some(msg) = messages.next().await? {
            iter_count += 1;

            if iter_count > target && msg.raw.id <= read_inbox_max_id {
                break;
            }

            raw_msgs.push(msg);
        }

        // Reactions in an album are attached to a single message, but they
        // apply to the whole group as far as the user is concerned. Collect
        // filtered group IDs first so every album member is dropped together.
        let mut filtered_groups: std::collections::HashSet<i64> = std::collections::HashSet::new();
        for msg in &raw_msgs {
            if has_excessive_negative_reactions(&msg.raw, filter) {
                if let Some(gid) = msg.raw.grouped_id {
                    filtered_groups.insert(gid);
                }
            }
        }

        let mut channel_posts: Vec<FetchedPost> = Vec::new();
        let mut group_indices: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();

        for msg in raw_msgs {
            match msg.raw.grouped_id {
                Some(gid) if filtered_groups.contains(&gid) => continue,
                None if has_excessive_negative_reactions(&msg.raw, filter) => continue,
                _ => {}
            }

            let text = msg.text().to_string();

            if let Some(gid) = msg.raw.grouped_id {
                if let Some(&idx) = group_indices.get(&gid) {
                    if !text.is_empty() && channel_posts[idx].text.is_empty() {
                        channel_posts[idx].text = text;
                    }
                    continue;
                }
                group_indices.insert(gid, channel_posts.len());
            }

            channel_posts.push(FetchedPost {
                channel_name: channel_name.clone(),
                text,
                date: msg.date(),
                view_count: msg.view_count(),
                id: None,
                link: None,
                source: Source::Telegram,
            });
        }
        posts.extend(channel_posts);

        // Mark channel as read
        if let Err(e) = client.mark_as_read(packed).await {
            eprintln!("Warning: failed to mark {} as read: {e}", channel_name);
        }
    }

    posts.sort_by(|a, b| b.date.cmp(&a.date));
    Ok(posts)
}

fn has_excessive_negative_reactions(raw: &tl::types::Message, config: &FilterConfig) -> bool {
    let reactions = match &raw.reactions {
        Some(tl::enums::MessageReactions::Reactions(r)) => r,
        _ => return false,
    };

    let top = reactions
        .results
        .iter()
        .map(|tl::enums::ReactionCount::Count(c)| c)
        .max_by_key(|c| c.count);

    let Some(top) = top else {
        return false;
    };

    if top.count < config.min_negative_reactions {
        return false;
    }

    let tl::enums::Reaction::Emoji(emoji) = &top.reaction else {
        return false;
    };

    config.negative_emojis.iter().any(|e| *e == emoji.emoticon)
}
