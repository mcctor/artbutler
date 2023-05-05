use crate::aggregator::{AggregatorStore, UserAggregator};
use crate::curator::Curator;
use crate::listings::reddit::{self, Api, Listing, Pagination, Seek, Subreddit};
use dotenvy::dotenv;

use log::info;
use reqwest::Client;
use std::sync::Arc;
use teloxide::dispatching::UpdateFilterExt;
use teloxide::prelude::{Dispatcher, Update};
use teloxide::{dptree, Bot};

use crate::auth::ClientManager;
use crate::content::Post;
use crate::listings::reddit::Listing::New;
use auth::ClientID;
use tokio::sync::Mutex;

mod aggregator;
mod artvault;
mod auth;
mod content;
mod curator;
mod imgproc;
mod listings;
mod schema;
mod telegram;

#[tokio::main]
async fn main() {
    dotenv().expect("include .env");
    pretty_env_logger::init();
    info!("Starting command bot...");

    let bot = Bot::from_env();

    let store = AggregatorStore::instance();
    let clients = Arc::new(Mutex::new(ClientManager::instance()));

    let store = Arc::new(Mutex::new(store));
    let handler =
        dptree::entry().branch(Update::filter_message().endpoint(telegram::listen_silence_handler));

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .dependencies(dptree::deps![store, clients])
        .build()
        .dispatch()
        .await;
}
