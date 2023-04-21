use crate::aggregator::AggregatorStore;
use crate::content::{ClientID, Post};
use crate::curator::Curator;
use crate::listings::reddit;
use log::{info, warn};
use reqwest::{Client, Url};
use std::sync::Arc;
use teloxide::payloads::SendPhotoSetters;
use teloxide::prelude::{Message, Requester, ResponseResult};
use teloxide::types::{InputFile, ParseMode};
use teloxide::Bot;
use tokio::spawn;
use tokio::sync::Mutex;

use crate::listings::reddit::Listing::New;
use crate::listings::reddit::{Api, Listing, Pagination, Subreddit};
use crate::telegram::Command::{Listen, Silence};

pub async fn listen_silence_handler(
    tg_bot: Bot,
    msg: Message,
    mut store: Arc<Mutex<AggregatorStore>>,
) -> ResponseResult<()> {
    let msg = msg.clone();
    let bot = tg_bot.clone();

    // let user_aggr = aggr_store.create(ClientID(msg.from().unwrap().id.0));
    let cmd = Command::parse(&msg);
    if cmd.is_err() {
        warn!(
            "Non-existent command requested by Client: {}",
            msg.from().unwrap().id
        );
        return Ok(());
    }
    match cmd.unwrap() {
        Listen { 0: listing } => {
            info!(
                "`/listen` command requested by userid: {} in chatid: {}",
                msg.from().unwrap().id,
                msg.chat.id
            );
            let mut guard = store.lock().await;
            let mut user = guard.find(msg.chat.id.0.into()).unwrap();

            let task = async move {
                user.attach_curator(Curator::from(Api::from(&Client::new())));
                user.add_listing(listing);

                while let Some(post) = user.curator.as_mut().unwrap().chan.1.recv().await {
                    let url = Url::parse(post.link.as_str()).unwrap();
                    let file = InputFile::url(url);
                    if let Ok(v) = bot
                        .send_photo(msg.chat.id, file)
                        .caption(format!("<i>{}</i>", post.title()))
                        .parse_mode(ParseMode::Html)
                        .await
                    {}
                    info!(
                        "Forwarded PostID: '{}' to UserID: '{}'",
                        post.id(),
                        msg.from().unwrap().id
                    );
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
