use anyhow::{bail, Context, Result};
use grammers_client::session::Session;
use grammers_client::types::Chat;
use grammers_client::{Client, Config, InitParams, SignInError};
use grammers_tl_types as tl;
use std::io::{self, Write};

use crate::config::{FilterConfig, TelegramConfig};

pub struct FetchedPost {
    pub channel_name: String,
    pub text: String,
    pub date: chrono::DateTime<chrono::Utc>,
    pub view_count: Option<i32>,
    pub id: Option<String>,
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

        let unread_count = match &dialog.raw {
            tl::enums::Dialog::Dialog(d) => d.unread_count,
            tl::enums::Dialog::Folder(_) => 0,
        };

        if unread_count == 0 {
            continue;
        }

        let channel_name = channel.title().to_string();
        let packed = channel.pack();

        let mut messages = client.iter_messages(packed).limit(unread_count as usize);
        while let Some(msg) = messages.next().await? {
            if has_excessive_negative_reactions(&msg.raw, filter) {
                continue;
            }

            let text = msg.text().to_string();
            posts.push(FetchedPost {
                channel_name: channel_name.clone(),
                text,
                date: msg.date(),
                view_count: msg.view_count(),
                id: None,
            });
        }

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

    for rc in &reactions.results {
        let tl::enums::ReactionCount::Count(count_data) = rc;
        if count_data.count >= config.min_negative_reactions {
            if let tl::enums::Reaction::Emoji(emoji) = &count_data.reaction {
                if config.negative_emojis.iter().any(|e| *e == emoji.emoticon) {
                    return true;
                }
            }
        }
    }

    false
}
