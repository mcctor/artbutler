use crate::aggregator::AggregatorStore;
use crate::auth::{BotClient, ClientID, ClientManager};
use crate::content::Post;
use crate::curator::Curator;
use crate::listings::reddit;
use log::{info, warn};

use crate::artvault::ArtVault;
use reqwest::{Client, Url};
use std::sync::Arc;
use teloxide::payloads::SendPhotoSetters;
use teloxide::prelude::{Message, Requester, ResponseResult};
use teloxide::types::CountryCode::AR;
use teloxide::types::{InputFile, Me, ParseMode};
use teloxide::Bot;
use tokio::spawn;
use tokio::sync::Mutex;

use crate::listings::reddit::Listing::New;
use crate::listings::reddit::{Api, Listing, Pagination, Subreddit};
use crate::telegram::Command::{Listen, Silence};

pub async fn listen_silence_handler(
    tg_bot: Bot,
    me: Me,
    msg: Message,
    mut store: Arc<Mutex<AggregatorStore>>,
    mut clients: Arc<Mutex<ClientManager>>,
) -> ResponseResult<()> {
    let msg = msg.clone();
    let bot = tg_bot.clone();

    let mut request_user = None;
    {
        let mut cli_manager = clients.lock().await;
        let v = cli_manager.get(msg.chat.id.0.into());
        if v.is_none() {
            cli_manager.add(BotClient {
                id: msg.chat.id.0.into(),
                username: msg.from().unwrap().username.clone(),
                is_user: true,
            });
        }
        request_user = Some(cli_manager.get(msg.chat.id.0.into()).unwrap().clone());
    }

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
