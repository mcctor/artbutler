use std::time::Duration;

use crate::{curator::{CuratorConfig, RedditCurator}, listings::reddit::{Listing, PaginationArg, Subreddit}};
use crate::aggregator::{ClientID, UserAggregator};

mod listings;
mod content;
mod curator;
mod aggregator;


#[tokio::main]
async fn main() {
    let mut curator = RedditCurator::from(
        CuratorConfig {
            sync_interval: Duration::from_secs(1),
            result_limit_per_cycle: 20
        }
    );
    curator.register_task(Subreddit::from("art"), Listing::New {
        params: PaginationArg::default()
    });
    curator.register_task(Subreddit::from("ArtPorn"), Listing::New {
        params: PaginationArg::default()
    });
    curator.attach_update_listener(Subreddit::from("dankmemes"));
    curator.attach_update_listener(Subreddit::from("memes"));

    let mut aggr = UserAggregator::new(ClientID(1), &mut curator);
    aggr.listen().await;
}
