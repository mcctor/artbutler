use std::sync::Arc;

use dotenvy::dotenv;
use log::info;
use teloxide::dispatching::{HandlerExt, UpdateFilterExt};
use teloxide::prelude::{Dispatcher, Update};
use teloxide::{dptree, Bot};
use tokio::sync::Mutex;

use crate::aggregator::AggregatorStore;
use crate::auth::ClientManager;
use crate::telegram::{ConfCommand, SubscribeCommand};

mod aggregator;
mod artvault;
mod auth;
mod content;
mod curator;
mod filters;
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
    let store = Arc::new(Mutex::new(AggregatorStore::instance()));
    let handler = Update::filter_message()
        .branch(
            dptree::entry()
                .filter_command::<ConfCommand>()
                .endpoint(telegram::configuration_cmd_handler),
        )
        .branch(
            dptree::entry()
                .filter_command::<SubscribeCommand>()
                .endpoint(telegram::listen_silence_handler),
        );

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .dependencies(dptree::deps![store])
        .build()
        .dispatch()
        .await;
}
