use std::sync::Arc;

use log::{info, warn};
use reqwest::{Client, Url};
use teloxide::macros::BotCommands;
use teloxide::payloads::SendPhotoSetters;
use teloxide::prelude::*;
use teloxide::prelude::{Message, Requester, ResponseResult};
use teloxide::types::{InputFile, Me, ParseMode};
use teloxide::Bot;
use tokio::spawn;
use tokio::sync::Mutex;

use crate::aggregator::AggregatorStore;
use crate::artvault::ArtVault;
use crate::auth::{BotClient, ClientID, ClientManager};
use crate::curator::Curator;
use crate::listings::reddit::{Api, Listing, Subreddit};
use crate::telegram::Command::{Listen, Silence};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "ConfCommand")]
pub enum ConfCommand {
    #[command(description = "initialize bot")]
    Start,

    #[command(description = "shows help")]
    Help,

    #[command(description = "user settings")]
    Settings,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "SubscribeCommand")]
pub enum SubscribeCommand {
    #[command(parse_with = "split", description = "generate a number within range")]
    Listen {
        subname: String,
        category: String,
    },
    Silence {
        subname: String,
    },
}

pub async fn configuration_cmd_handler(
    tg_bot: Bot,
    msg: Message,
    cmd: ConfCommand,
) -> Result<(), teloxide::RequestError> {
    match cmd {
        ConfCommand::Start => {
            let mut cli_mgr = ClientManager::instance();
            if let Some(registered) = cli_mgr.get(ClientID::from(msg.from().unwrap().id.0 as i64)) {
                tg_bot
                    .send_message(
                        ChatId(registered.id.id()),
                        format!("Welcome back, {}!", registered.username.as_ref().unwrap()),
                    )
                    .await
                    .unwrap();
            } else {
                let botclient = BotClient {
                    id: ClientID::from(msg.from().unwrap().id.0 as i64),
                    username: msg.from().unwrap().username.clone(),
                    is_user: !msg.from().unwrap().is_bot,
                };
                cli_mgr.add(botclient);
            }
            Ok(())
        }
        ConfCommand::Help => Ok(()),
        ConfCommand::Settings => Ok(()),
    }
}

pub async fn listen_silence_handler(
    tg_bot: Bot,
    msg: Message,
    store: Arc<Mutex<AggregatorStore>>,
    clients: Arc<Mutex<ClientManager>>,
) -> ResponseResult<()> {
    let msg = msg.clone();
    let bot = tg_bot.clone();

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
                    let mut vault = ArtVault::instance();
                    let is_post = vault.fetch(post.id());
                    if is_post.is_some() {
                        continue;
                    }

                    let url = Url::parse(post.media_href.as_str()).unwrap();
                    let file = InputFile::url(url);

                    if let Ok(_) = bot
                        .send_photo(msg.chat.id, file)
                        .caption(format!("<i>{}</i>", post.title()))
                        .parse_mode(ParseMode::Html)
                        .await
                    {
                        vault.save(&post);
                    }
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
