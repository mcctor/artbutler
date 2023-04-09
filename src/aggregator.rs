use log::info;
use std::collections::VecDeque;

use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::content::Post;
use crate::curator::Curator;
use crate::listings::source::ListingSource;

pub trait Filter {
    fn check(&self, post: &Post) -> bool;
}

pub struct VoteCountFilter;

impl Filter for VoteCountFilter {
    fn check(&self, post: &Post) -> bool {
        todo!()
    }
}

pub struct BlockedFilter;

impl Filter for BlockedFilter {
    fn check(&self, post: &Post) -> bool {
        todo!()
    }
}

pub struct SimilarFilter;

impl Filter for SimilarFilter {
    fn check(&self, post: &Post) -> bool {
        todo!()
    }
}

#[derive(Copy, Clone, PartialEq)]
pub struct ClientID(pub u64);

pub struct UserAggregator<S> {
    id: ClientID,
    curator: Curator<S>,
    cache: VecDeque<Post>,
    pub chan: (Sender<Post>, Receiver<Post>),
}

impl<S> UserAggregator<S>
where
    S: ListingSource + Send + Sync + 'static,
{
    pub fn new(id: ClientID, curator: Curator<S>) -> Self {
        let (tx, rcv) = channel(10);
        UserAggregator {
            id,
            curator,
            cache: VecDeque::with_capacity(5),
            chan: (tx, rcv),
        }
    }

    pub async fn listen(&mut self) {
        while let Some(post) = self.curator.chan.1.recv().await {
            self.cache.push_back(post.clone());
            info!("{:?}", post.clone());
            self.chan.0.send(post).await.unwrap();
        }
    }
}

pub struct AggregatorStore<S> {
    store: Vec<UserAggregator<S>>,
}

impl<S> AggregatorStore<S>
where
    S: Send + Sync,
{
    pub fn new() -> Self {
        Self { store: vec![] }
    }

    pub fn find_for(&mut self, client: ClientID) -> Option<&mut UserAggregator<S>> {
        self.store.iter_mut().find_map(|a| {
            if a.id == client {
                return Some(a);
            }
            None
        })
    }

    pub fn create(&mut self, client_id: ClientID) -> &mut UserAggregator<S> {
        let is_found = self.find_for(client_id);
        if is_found.is_some() {
            return is_found.unwrap();
        }
        // let aggr = UserAggregator::new(client_id);
        // self.store.push(aggr);
        // self.store.get_mut(self.store.len() - 0).unwrap()
        todo!()
    }
}
