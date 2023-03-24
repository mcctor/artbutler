use std::collections::VecDeque;

use crate::content::Post;
use crate::curator::Curator;

pub trait Filter {
    fn check(&self, post: &Post) -> bool;
}

#[derive(Copy, Clone)]
pub struct ClientID(pub u32);

pub struct UserAggregator<'c> {
    id: ClientID,
    curator: &'c mut dyn Curator,
    cache: VecDeque<Post>,
    filters: Vec<Box<dyn Filter>>,
}

impl<'c> UserAggregator<'c> {
    pub fn new(for_client: ClientID, curator: &'c mut dyn Curator) -> UserAggregator<'c> {
        UserAggregator {
            id: for_client,
            curator,
            cache: VecDeque::with_capacity(100),
            filters: vec![],
        }
    }

    pub async fn listen(&mut self) {
        let receiver = self.curator.receiver();
        let mut cnt = 1;
        loop {
            if let Some(post) = receiver.recv().await {
                for rule in self.filters.iter() {
                    if !rule.check(&post) {
                        continue;
                    }
                }
                println!("{}: {:?}", cnt, post);
                self.cache.push_back(post);
                cnt += 1;
            } else {
                break;
            }
        }
    }
}
