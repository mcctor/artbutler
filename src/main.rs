use std::sync::Arc;

use dotenvy::dotenv;
use log::info;
use teloxide::{Bot, dptree};
use teloxide::dispatching::UpdateFilterExt;
use teloxide::prelude::{Dispatcher, Update};
use tokio::sync::Mutex;

use crate::aggregator::AggregatorStore;
use crate::botclient::ClientManager;

mod aggregator;
mod botclient;
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
