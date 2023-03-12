use std::env;
use dotenvy::dotenv;
use teloxide::{prelude::*, utils::command::BotCommands};

use crate::{curator::{RedditCurator}, listings::reddit::{Listing, PaginationArg, Subreddit}};
use crate::aggregator::{ClientID, UserAggregator};
use crate::curator::Curator;
use crate::listings::reddit::Reddit;

mod listings;
mod content;
mod curator;
mod aggregator;


#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Quick access butler commands:")]
enum ButlerCommands {
    #[command(description = "handle a username and an age.")]
    Livefeed(String),

    #[command(description = "handle a username and an age.")]
    Listen,

    #[command(description = "handle a username and an age.")]
    Silence
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "User prefs & configurations:")]
enum SettingCommands {
    UpdateRate
}


#[tokio::main]
async fn main() {
    dotenv().expect("include .env");
    pretty_env_logger::init();
    log::info!("Starting command bot...");

    let bot = Bot::from_env();

    ButlerCommands::repl(bot, answer).await;
}

async fn answer(bot: Bot, msg: Message, cmd: ButlerCommands) -> ResponseResult<()> {
    let api = Reddit::new();
    let mut curator = RedditCurator::from(api);
    curator.attach_listener(Subreddit::from("art"));
    curator.attach_listener(Subreddit::from("artporn"));
    // curator.attach_listener(Subreddit::from("IllustrativeArt"));

    let mut cnt = 0;
    loop {
        if let Some(post) = curator.receiver().recv().await {
            cnt += 1;
            bot.send_message(msg.chat.id, format!("{}: {:?}", cnt, post.media_href)).await.unwrap();
        }
    }
}