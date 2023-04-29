use std::sync::Arc;
use std::time::Duration;

use diesel::result::{DatabaseErrorKind, Error};
use log::{info, warn};
use reqwest::Url;
use teloxide::payloads::SendPhotoSetters;
use teloxide::prelude::{ChatId, Message, Requester, ResponseResult};
use teloxide::types::{InputFile, ParseMode};
use teloxide::Bot;
use tokio::spawn;
use tokio::sync::Mutex;
use tokio::time::{sleep_until, Instant};

use crate::botclient::ClientManager;
use crate::listings::reddit::{Listing, Subreddit};
use crate::telegram::Command::{Listen, Silence};

pub async fn listen_silence_handler(
    tg_bot: Bot,
    msg: Message,
    cli_mgr: Arc<Mutex<ClientManager>>,
) -> ResponseResult<()> {
    let msg = msg.clone();
    let bot = tg_bot.clone();
    let chat_id = msg.chat.id.0;

    let mut request_user = None;
    {
        let mut cli_manager = cli_mgr.lock().await;
        if let Some(client) = cli_manager.get(chat_id.into()).await {
            request_user = Some(client);
        } else {
            let client = cli_manager
                .add_new_user(
                    chat_id.into(),
                    {
                        if let Some(v) = msg.from() {
                            v.username.clone()
                        } else {
                            None
                        }
                    },
                    {
                        if let Some(v) = msg.from() {
                            !v.is_bot
                        } else {
                            false
                        }
                    },
                )
                .await
                .unwrap();
            request_user = Some(client);
        }
    }
    let client = request_user.unwrap();
    let user_info = &client.0;

    let cmd = Command::parse(&msg);
    if cmd.is_err() {
        warn!("Non-existent command requested by {:?}", user_info.id());
        return Ok(());
    }

    match cmd.unwrap() {
        Listen { 0: listing } => {
            info!(
                "`/listen` command requested by {:?} in ChatID: {}",
                user_info.id(),
                chat_id
            );

            let aggr = client.1.clone();
            let task = async move {
                let mut aggr = aggr.lock().await;
                let res = aggr.save_to_db(&listing);

                if let Err(Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) = res {
                    bot.send_message(msg.chat.id, "Already listening to that listing")
                        .await
                        .unwrap();
                    return;
                } else {
                    aggr.add_listing(Arc::new(Mutex::new(listing)));
                }

                while let Some(post) = aggr.curator.chan.1.recv().await {
                    let url = Url::parse(post.link.as_str()).unwrap();
                    let file = InputFile::url(url);
                    if let Ok(v) = bot
                        .send_photo(ChatId(chat_id), file)
                        .caption(format!("<i>{}</i>", post.title()))
                        .parse_mode(ParseMode::Html)
                        .await
                    {}
                    info!("Forwarded PostID: '{}' to UserID: '{}'", post.id(), chat_id);
                    let mut cache_guard = aggr.cache.lock().await;
                    cache_guard.push_back(post);
                }
            };
            spawn(task);
        }

        Silence { 0: sub } => {
            info!(
                "`/silence` command requested by userid: {}",
                msg.from().unwrap().id
            );
            // curator.detach_listeners(&sub);
        }
    }

    Ok(())
}

pub async fn init_listing_listeners(bot: Bot) -> Arc<Mutex<ClientManager>> {
    let cli_mgr = ClientManager::instance().await.unwrap();
    let cli_mgr = Arc::new(Mutex::new(cli_mgr));

    for listing in &cli_mgr.lock().await.existing {
        let bot = bot.clone();
        let aggr = listing.1.clone();
        let botclient = listing.0.clone();

        let task = async move {
            loop {
                let mut aggr = aggr.lock().await;
                if let Some(post) = aggr.curator.chan.1.recv().await {
                    let url = Url::parse(post.link.as_str()).unwrap();
                    let file = InputFile::url(url);
                    if let Ok(v) = bot
                        .send_photo(ChatId(botclient.id()), file)
                        .caption(format!("<i>{}</i>", post.title()))
                        .parse_mode(ParseMode::Html)
                        .await
                    {}
                    info!(
                        "Forwarded PostID: '{}' to UserID: '{}'",
                        post.id(),
                        botclient.id()
                    );
                    let mut cache_guard = aggr.cache.lock().await;
                    cache_guard.push_back(post);
                } else {
                    break;
                }
            }
        };
        spawn(task);
    }

    cli_mgr.clone()
}

#[derive(Debug)]
struct ArgumentError;

enum Command {
    Listen(Listing),
    Silence(Subreddit),
}

impl Command {
    fn parse(msg: &Message) -> Result<Command, ArgumentError> {
        let values = msg.text().unwrap().split(" ").collect::<Vec<&str>>();
        if values.is_empty() {
            return Err(ArgumentError);
        }

        let cmd = values.first().unwrap();
        match *cmd {
            "/listen" => {
                if let Some(sub) = values.get(1) {
                    if let Some(listing) = values.get(2) {
                        let listing = Listing::from(listing, Subreddit::from(sub));
                        return Ok(Listen(listing));
                    }
                }
                Err(ArgumentError)
            }
            "/silence" => {
                if let Some(sub) = values.get(1) {
                    return Ok(Silence(Subreddit::from(sub)));
                }
                Err(ArgumentError)
            }
            _ => Err(ArgumentError),
        }
    }
}

impl ToString for Command {
    fn to_string(&self) -> String {
        match self {
            Listen { .. } => "/listen".to_string(),
            Silence { .. } => "/silence".to_string(),
        }
    }
}
