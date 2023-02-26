use std::collections::VecDeque;
use std::time::Duration;

use tokio::{join, spawn};
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;

use crate::content::Post;
use crate::curator::{Curator, CuratorConfig, RedditCurator};
use crate::listings::reddit::{Listing, PaginationArg, Subreddit};

struct FilterRule;

#[derive(Copy, Clone)]
pub struct ClientID(pub u32);

pub struct UserAggregator<'a> {
    id: ClientID,
    curator: &'a mut RedditCurator,
    cache: VecDeque<Post>,
    filters: Vec<FilterRule>
}

impl<'a> UserAggregator<'a> {
    pub fn new(for_client: ClientID, curator: &'a mut RedditCurator) -> UserAggregator {
        UserAggregator {
            id: for_client,
            curator,
            cache: VecDeque::with_capacity(100),
            filters: vec![]
        }
    }

    pub async fn listen(&mut self) {
        let receiver = self.curator.receiver();
        let mut cnt = 0;
        loop {
            if let Some(post) = receiver.recv().await {
                cnt += 1;
                println!("{}: {:?}", cnt, post);
                self.cache.push_back(post);
            } else {
                break;
            }
        }
    }
}

#[tokio::test]
async fn test_user_aggregator() {
    let mut curator = RedditCurator::from(
        CuratorConfig {
            sync_interval: Duration::from_secs(5),
            result_limit_per_cycle: 5
        }
    );
    curator.register_task(Subreddit::from("popular"), Listing::New {
        params: PaginationArg::default()
    });
    curator.attach_update_listener(Subreddit::from("popular"));

    let mut aggr = UserAggregator::new(ClientID(1), &mut curator);
    aggr.listen().await;
}