use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::{spawn, sync::mpsc::{channel, Receiver}};
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep_until};

use crate::{
    content::Post,
    listings::reddit::{Listing, PaginationArg, Reddit, Subreddit},
};
use crate::listings::reddit::Seek;

pub type Listeners = HashMap<String, Vec<JoinHandle<()>>>;

pub const SYNC_INTERVAL_MAX: u64 = 254;
pub const SYNC_INTERVAL_DEFAULT: u64 = 1;
pub const QUERY_RESULT_LIMIT: u8 = 5;


pub trait Curator {
    fn receiver(&mut self) -> &mut Receiver<Post>;
}

pub struct RedditCurator {
    api: Arc<Mutex<Reddit>>,
    tasks: Listeners,
    chan: (Sender<Post>, Receiver<Post>),
}

impl RedditCurator {
    pub fn from(api: Reddit) -> Self {
        let (tx, rcv) = channel(10);
        let api = Arc::new(Mutex::new(api));
        RedditCurator {
            api,
            tasks: HashMap::new(),
            chan: (tx, rcv),
        }
    }

    pub fn attach_update_listener(&mut self, sub: Subreddit) {
        let tx = self.chan.0.clone();
        let subreddit = sub.clone();
        let api = self.api.clone();

        let mut listing = Listing::New {
            params: PaginationArg {
                cursor_anchor: Seek::Before { post: Post::empty() },
                limit: QUERY_RESULT_LIMIT,
                seen_count: 0,
                show_rules: "null".to_string(),
            }
        };

        let mut sync_interval = SYNC_INTERVAL_DEFAULT as u64;
        let listener = async move {
            loop {
                let mut synced_posts = vec![];
                {
                    let mut guard = api.lock().await;
                    let task = guard.retrieve_posts(
                        &subreddit,
                        &mut listing,
                    );

                    let post_res = task.await;
                    if post_res.is_err() {
                        continue;
                    }
                    synced_posts.append(&mut post_res.unwrap())
                }

                if synced_posts.len() != 0 {
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
            }
        };

        let listener_task = spawn(listener);
        if let Some(v) = self.tasks.get_mut(sub.name().as_str()) {
            v.push(listener_task);
        } else {
            self.tasks.insert(sub.name(), vec![listener_task]);
        }
    }

    pub fn detach_listeners(&mut self, sub: Subreddit) {
        todo!();
    }
}

impl Curator for RedditCurator {
    fn receiver(&mut self) -> &mut Receiver<Post> {
        &mut self.chan.1
    }
}
