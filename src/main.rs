use dotenvy::dotenv;
use std::collections::{HashMap, VecDeque};
use teloxide::prelude::*;

mod aggregator;
mod content;
mod curator;
mod listings;
mod telegram;

#[tokio::main]
async fn main() {
    dotenv().expect("include .env");
    pretty_env_logger::init();
    log::info!("Starting command bot...");

    let bot = Bot::from_env();
    let handler =
        dptree::entry().branch(Update::filter_message().endpoint(telegram::listen_silence_handler));

    Dispatcher::builder(bot, handler)
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
