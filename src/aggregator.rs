use std::collections::VecDeque;

use crate::content::Post;
use crate::curator::{Curator, RedditCurator};

struct FilterRule;

#[derive(Copy, Clone)]
pub struct ClientID(pub u32);

pub struct UserAggregator<'a> {
    id: ClientID,
    curator: &'a mut dyn Curator,
    cache: VecDeque<Post>,
    filters: Vec<FilterRule>
}

impl<'a> UserAggregator<'a> {
    pub fn new(for_client: ClientID, curator: &'a mut dyn Curator) -> UserAggregator<'a> {
        UserAggregator {
            id: for_client,
            curator,
            cache: VecDeque::with_capacity(100),
            filters: vec![]
        }
    }

    pub async fn listen(&mut self) {
        let receiver = self.curator.receiver();
        loop {
            if let Some(post) = receiver.recv().await {
                println!("{:?}", post);
                self.cache.push_back(post);
            } else {
                break;
            }
        }
    }
}
