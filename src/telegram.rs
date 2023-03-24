use reqwest::Url;
use teloxide::prelude::{Message, Requester, ResponseResult};
use teloxide::Bot;
use teloxide::types::InputFile;
use tokio::spawn;

use crate::curator::{Curator, RedditCurator};
use crate::listings::reddit::{Listing, Subreddit};
use crate::telegram::Command::{Listen, Silence};

pub async fn listen_silence_handler(bot: Bot, msg: Message) -> ResponseResult<()> {
    let bot = bot.clone();
    let msg = msg.clone();
    // TODO: assumption is, one curator per client is fetched by this method.
    let mut curator = RedditCurator::new();
    let cmd = Command::parse(&msg).expect("unable to parse command from message");
    match cmd {
        Listen { 0: listing } => {
            let curation_task = async move {
                curator.attach_listener(listing);
                let mut cnt = 0;
                while let Some(post) = curator.receiver().recv().await {
                    cnt += 1;

                    let path = Url::parse(post.media_href.as_str()).unwrap();
                    let file = InputFile::url(path);
                    if let Ok(v) = bot.send_photo(msg.chat.id, file).await {
                    }
                }
            };
            spawn(curation_task);
        }

        Silence { 0: listing } => {
            curator.detach_listeners(&listing);
        }
    }

    Ok(())
}

#[derive(Debug)]
struct ArgumentError;

enum Command {
    Listen(Listing),
    Silence(Listing),
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
            "/silence" => Err(ArgumentError),
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
