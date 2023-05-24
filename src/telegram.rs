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

use crate::listings::reddit::{Api, Listing};

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
            if let Some(registered) = cli_mgr.get(msg.from().unwrap().id.into()) {
                tg_bot
                    .send_message(
                        ChatId(registered.id.id()),
                        format!("Welcome back, {}!", registered.username.as_ref().unwrap()),
                    )
                    .await
                    .unwrap();
            } else {
                let botclient = BotClient {
                    id: msg.from().unwrap().id.into(),
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
    cmd: SubscribeCommand,
    store: Arc<Mutex<AggregatorStore>>,
) -> ResponseResult<()> {
    let msg = msg.clone();
    let bot = tg_bot.clone();

    match cmd {
        SubscribeCommand::Listen { subname, category } => {
            let listing = Listing::from(category.as_str(), subname.into());
            info!(
                "`/listen` command requested by userid: {} in chatid: {}",
                msg.from().unwrap().id,
                msg.chat.id
            );
            let mut guard = store.lock().await;
            let mut user = guard.find::<Api>(msg.from().unwrap().id.into()).unwrap();

            let task = async move {
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
        SubscribeCommand::Silence { subname } => {
            info!(
                "`/silence` command requested by userid: {}",
                msg.from().unwrap().id
            );
        }
    }

    Ok(())
}
