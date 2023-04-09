use crate::aggregator::{AggregatorStore, ClientID, UserAggregator};
use crate::curator::Curator;
use crate::listings::reddit::{self, Api, Listing, Pagination, Seek, Subreddit};
use dotenvy::dotenv;

use log::info;
use reqwest::Client;
use std::sync::Arc;
use teloxide::dispatching::UpdateFilterExt;
use teloxide::prelude::{Dispatcher, Update};
use teloxide::{dptree, Bot};

use crate::content::Post;
use crate::listings::reddit::Listing::New;
use tokio::sync::Mutex;

mod aggregator;
mod content;
mod curator;
mod imgproc;
mod listings;
mod telegram;

#[tokio::main]
async fn main() {
    dotenv().expect("include .env");
    pretty_env_logger::init();
    info!("Starting command bot...");

    let bot = Bot::from_env();
    let handler =
        dptree::entry().branch(Update::filter_message().endpoint(telegram::listen_silence_handler));

    let curation = Box::new(Curator::from(Api::from(&Client::new())));
    let store: Arc<AggregatorStore<Api>> = Arc::new(AggregatorStore::new());
    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .dependencies(dptree::deps![store])
        .build()
        .dispatch()
        .await;
}
