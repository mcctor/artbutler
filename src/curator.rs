use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use log::{error, info, warn};
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{sleep_until, Instant};
use tokio::{
    spawn,
    sync::mpsc::{channel, Receiver},
};

use crate::listings::reddit::{Seek, Subreddit};
use crate::{content::Post, listings::reddit::Listing};

use crate::listings::source::ListingSource;

pub const SYNC_INTERVAL_MAX: u64 = 32;

pub const SYNC_INTERVAL_DEFAULT: u64 = 1;

struct Buffer {
    buf: VecDeque<Post>,
    size: usize,
}

impl Buffer {
    fn new(len: usize) -> Self {
        if len == 0 {
            panic!("buffer must be of size > 0");
        }
        Self {
            buf: VecDeque::with_capacity(len),
            size: len,
        }
    }

    fn is_full(&self) -> bool {
        !(self.buf.len() < self.size)
    }

    fn difference(&self, other: VecDeque<Post>) -> Vec<Post> {
        let mut buf_set = HashSet::new();

        for post in self.buf.iter() {
            buf_set.insert(post);
        }

        let mut diff = Vec::new();
        for post in other {
            if !buf_set.contains(&post) {
                diff.push(post)
            }
        }

        diff
    }

    fn insert(&mut self, p: Post) {
        if self.is_full() {
            self.buf.push_back(p);
            self.buf.pop_front();
            return;
        }
        self.buf.push_back(p);
    }
}

pub struct Curator<T> {
    src: T,
    curations: Vec<JoinHandle<()>>,
    pub chan: (Sender<Post>, Receiver<Post>),
}

impl<T: ListingSource> Curator<T> {
    pub fn from(src: T) -> Self {
        Curator {
            src,
            curations: vec![],
            chan: channel(5),
        }
    }

    pub fn spawn_for(&mut self, listing: Arc<Mutex<Listing>>) {
        let api = self.src.clone();
        let tx = self.chan.0.clone();
        let task = spawn(Self::listing_listener(
            api,
            tx,
            listing,
            SYNC_INTERVAL_DEFAULT as u64,
        ));
        self.curations.push(task);
    }

    async fn listing_listener(
        mut api: T,
        tx: Sender<Post>,
        listing: Arc<Mutex<Listing>>,
        mut sync_interval: u64,
    ) {
        let mut timeout_cnt = 0;
        let mut buffer = Buffer::new(10);

        loop {
            let mut synced_posts = VecDeque::new();
            let mut sub = "".into();
            {
                let mut listing_guard = listing.lock().await;
                sub = listing_guard.subreddit().clone();

                let res = api.retrieve_posts(&mut listing_guard).await;
                if res.is_err() {
                    error!("couldn't retrieve posts: {}", res.err().unwrap());
                    warn!("Retrying post retrieval in 10s");

                    sleep_until(Instant::now() + Duration::from_secs(10)).await;
                    continue;
                }

                let posts = res.unwrap();
                match listing_guard.paginator().cursor() {
                    Seek::After { .. } => {
                        for post in posts {
                            synced_posts.push_back(post);
                        }
                    }
                    Seek::Back { .. } => {
                        for post in posts {
                            synced_posts.push_front(post);
                        }
                    }
                }
            }

            let new_posts = buffer.difference(synced_posts);
            if !new_posts.is_empty() {
                info!(
                    "{} new post(s) found for `r/{}`! Resetting wait interval",
                    new_posts.len(),
                    sub.name(),
                );

                for post in new_posts {
                    buffer.insert(post.clone());
                    let sent = tx.send(post).await;
                    if sent.is_err() {
                        continue;
                    }
                }

                sync_interval = SYNC_INTERVAL_DEFAULT;
                continue;
            } else {
                if sync_interval < SYNC_INTERVAL_MAX {
                    sync_interval *= 2;
                }
                info!(
                    "No new post since last poll for `r/{}`, \
                         increased wait interval to {}s",
                    sub.name(),
                    sync_interval
                );
            }

            sleep_until(Instant::now() + Duration::from_secs(sync_interval)).await;
            if sync_interval >= SYNC_INTERVAL_MAX {
                if timeout_cnt == 2 {
                    let mut synced_posts = VecDeque::new();
                    {
                        info!("Polling timeout for `r/{}`, retrying ...", sub.name());
                        let mut listing_guard = listing.lock().await;
                        match listing_guard.paginator().cursor() {
                            Seek::After { .. } => {
                                let mut deck = VecDeque::new();
                                deck.push_back(Post::empty());
                                listing_guard.update_paginator_cache(&deck);

                                let res = api.retrieve_posts(&mut listing_guard).await.unwrap();
                                for post in res {
                                    synced_posts.push_back(post);
                                }
                            }
                            Seek::Back { .. } => {
                                info!("Finished polling back, no more posts. Exiting ...");
                                break;
                            }
                        }
                    }

                    let new_posts = buffer.difference(synced_posts);
                    for post in new_posts {
                        buffer.insert(post.clone());
                        let sent = tx.send(post).await;
                        if sent.is_err() {
                            continue;
                        }
                    }

                    sync_interval = SYNC_INTERVAL_DEFAULT;
                    timeout_cnt = 0;
                } else {
                    timeout_cnt += 1;
                }
            }
        }
    }
}
