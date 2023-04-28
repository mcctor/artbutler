use dotenvy::dotenv;
use log::info;
use teloxide::dispatching::UpdateFilterExt;
use teloxide::prelude::*;
use teloxide::{dptree, Bot};

mod aggregator;
mod botclient;
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
    let clients = telegram::init_listing_listeners(bot.clone()).await;

    let handler =
        dptree::entry().branch(Update::filter_message().endpoint(telegram::listen_silence_handler));

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .dependencies(dptree::deps![clients])
        .build()
        .dispatch()
        .await;
}
