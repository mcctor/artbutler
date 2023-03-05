use crate::{curator::{RedditCurator}, listings::reddit::{Listing, PaginationArg, Subreddit}};
use crate::aggregator::{ClientID, UserAggregator};
use crate::listings::reddit::Reddit;

mod listings;
mod content;
mod curator;
mod aggregator;


#[tokio::main]
async fn main() {
    let api = Reddit::new();
    let mut curator = RedditCurator::from(api);
    // curator.register_task(Subreddit::from("art"), Listing::New {
    //     params: PaginationArg::default()
    // });
    // curator.register_task(Subreddit::from("pics"), Listing::New {
    //     params: PaginationArg::seek_back()
    // });
    curator.attach_update_listener(Subreddit::from("art"));
    // curator.attach_update_listener(Subreddit::from("artporn"));
    // curator.attach_update_listener(Subreddit::from("memes"));


    let mut aggr = UserAggregator::new(ClientID(1), &mut curator);
    aggr.listen().await;
}
