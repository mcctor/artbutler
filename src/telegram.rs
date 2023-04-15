use crate::aggregator::{AggregatorStore, ClientID};
use crate::content::Post;
use crate::curator::Curator;
use crate::listings::reddit;
use log::info;
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
    aggr_store: Arc<AggregatorStore<Api>>,
) -> ResponseResult<()> {
    let msg = msg.clone();
    let bot = tg_bot.clone();

    // let user_aggr = aggr_store.create(ClientID(msg.from().unwrap().id.0));
    let cmd = Command::parse(&msg).expect("unable to parse command from message");
    match cmd {
        Listen { 0: listing } => {
            info!(
                "`/listen` command requested by userid: {} in chatid:{}",
                msg.from().unwrap().id,
                msg.chat.id
            );
            let task = async move {
                let mut reddit = reddit::Api::from(&Client::new());
                reddit.authenticate_or_refresh().await.unwrap();

                let mut curator = Curator::from(reddit);
                curator.spawn_for(Arc::new(Mutex::new(listing)));

                while let Some(post) = curator.chan.1.recv().await {
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
