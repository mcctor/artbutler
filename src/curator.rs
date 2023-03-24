use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{sleep_until, Instant};
use tokio::{
    spawn,
    sync::mpsc::{channel, Receiver},
};

use crate::{
    content::Post,
    listings::reddit::{Listing, Reddit, Seek, Subreddit},
};

pub type Listeners = HashMap<String, Vec<JoinHandle<()>>>;

pub const SYNC_INTERVAL_MAX: u64 = 128;

pub const SYNC_INTERVAL_DEFAULT: u64 = 1;

pub const QUERY_RESULT_LIMIT: u8 = 5;

pub trait Curator {
    fn receiver(&mut self) -> &mut Receiver<Post>;
}

pub struct RedditCurator {
    api: Arc<Mutex<Reddit>>,
    task_groups: Listeners,
    chan: (Sender<Post>, Receiver<Post>),
}

impl RedditCurator {
    pub fn new() -> Self {
        let (tx, rcv) = channel(10);
        RedditCurator {
            api: Arc::new(Mutex::new(Reddit::new())),
            task_groups: HashMap::new(),
            chan: (tx, rcv),
        }
    }

    pub fn attach_listener(&mut self, mut listing: Listing) {
        let tx = self.chan.0.clone();
        let subreddit = listing.subreddit();
        let api = self.api.clone();
        let mut sync_interval = SYNC_INTERVAL_DEFAULT as u64;

        let listener = async move {
            let mut timeout_cnt = 0;
            loop {
                let mut synced_posts = vec![];
                {
                    let mut guard = api.lock().await;
                    let res = guard.retrieve_posts(&mut listing).await;
                    if res.is_err() {
                        continue;
                    }
                    synced_posts.append(&mut res.unwrap())
                }

                if !synced_posts.is_empty() {
                    synced_posts.reverse();
                    for post in synced_posts {
                        tx.send(post).await.unwrap();
                    }
                    sync_interval = SYNC_INTERVAL_DEFAULT;

                } else {
                    if sync_interval < SYNC_INTERVAL_MAX {
                        sync_interval = sync_interval * 2;
                    }
                }

                sleep_until(Instant::now() + Duration::from_secs(sync_interval)).await;
                if sync_interval == SYNC_INTERVAL_MAX {
                    if timeout_cnt > 3 {
                        // reset listing anchor
                        listing.set_anchor_post(Post::empty());
                        timeout_cnt = 0;
                    }
                    timeout_cnt += 1;
                }
            }
        };

        let listener = spawn(listener);
        if let Some(group) = self.task_groups.get_mut(subreddit.name().as_str()) {
            group.push(listener);
        } else {
            self.task_groups.insert(subreddit.name(), vec![listener]);
        }
    }

    pub fn detach_listeners(&mut self, listing: &Listing) -> Vec<JoinHandle<()>> {
        self.task_groups
            .remove(listing.subreddit().name().as_str())
            .unwrap_or(vec![])
    }
}

impl Curator for RedditCurator {
    fn receiver(&mut self) -> &mut Receiver<Post> {
        &mut self.chan.1
    }
}
