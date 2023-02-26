use std::collections::HashMap;
use std::ops::Sub;
use std::time::Duration;

use reqwest::Client;
use tokio::{spawn, sync::mpsc::{self, Receiver}, time::{Instant, sleep_until}};
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;

use crate::{
    content::Post,
    listings::reddit::{Listing, PaginationArg, Reddit, Subreddit},
};
use crate::listings::reddit::Seek;

type CuratorTasks = Vec<(Subreddit, Listing, Option<JoinHandle<()>>)>;

pub trait Curator {
    fn receiver(&mut self) -> &mut Receiver<Post>;
}

#[derive(Clone, Copy)]
pub struct CuratorConfig {
    pub sync_interval: Duration,
    pub result_limit_per_cycle: u8,
}

pub struct RedditCurator {
    cli: Client,
    tasks: CuratorTasks,
    conf: CuratorConfig,
    listeners: HashMap<String, Vec<JoinHandle<()>>>,
    tx: Sender<Post>,
    rcv: Receiver<Post>,
}

impl RedditCurator {
    pub fn from(conf: CuratorConfig) -> Self {
        let (tx, rcv) = mpsc::channel(10);
        RedditCurator {
            cli: Client::new(),
            tasks: Vec::new(),
            conf,
            listeners: HashMap::new(),
            tx,
            rcv,
        }
    }

    pub fn register_task(&mut self, from_src: Subreddit, of: Listing) {
        let task = Self::exec_retrieval_task(
            self.cli.clone(),
            self.tx.clone(),
            from_src.clone(),
            of.clone(),
        );
        let task = spawn(task);
        self.tasks.push((from_src, of, Some(task)));
    }

    pub fn attach_update_listener(&mut self, sub: Subreddit) {
        let cli = self.cli.clone();
        let tx = self.tx.clone();
        let subreddit = sub.clone();

        let mut listing = Listing::New {
            params: PaginationArg {
                cursor_anchor: Seek::Before { post_id: "null".to_string() },
                limit: self.conf.result_limit_per_cycle,
                seen_count: 0,
                show_rules: "null".to_string(),
            }
        };

        let sync_interval = self.conf.sync_interval;
        let listener_future = async move {
            loop {
                let task = Reddit::retrieve_posts(
                    &cli,
                    &subreddit,
                    &mut listing,
                );
                let mut synced_posts = task.await;
                if synced_posts.len() != 0 {
                    synced_posts.reverse();
                    for post in synced_posts {
                        tx.send(post).await.unwrap();
                    }
                }
                // TODO: Adopt sleep time according to number of retries with no results
                sleep_until(Instant::now() + sync_interval).await;
            }
        };
        let listener = spawn(listener_future);

        if let Some(v) = self.listeners.get_mut(sub.name().as_str()) {
            v.push(listener);
        } else {
            self.listeners.insert(sub.name(), vec![listener]);
        }
    }

    pub fn detach_listeners(&mut self, sub: Subreddit) {
        todo!();
    }

    pub fn clear_all_tasks(&mut self, for_sub: &Subreddit) {
        let mut idx_found = vec![];
        let mut cnt = -1;

        loop {
            let mut task_iter = self.tasks.iter_mut();
            if let Some(index) =
                task_iter.position(|(sub, _, _)| {
                    if *for_sub == *sub {
                        true
                    } else {
                        false
                    }
                })
            {
                idx_found.push(index);
                cnt += 1;
            } else {
                break;
            };

            self.tasks.swap_remove(idx_found[cnt as usize]);
        }
    }

    async fn exec_retrieval_task(
        cli: Client,
        tx: Sender<Post>,
        sub: Subreddit,
        mut listing: Listing,
    ) {
        loop {
            let task = Reddit::retrieve_posts(
                &cli,
                &sub,
                &mut listing,
            );
            let mut awaited_posts = task.await;
            if awaited_posts.len() == 0 {
                break;
            }
            awaited_posts.reverse();
            for post in awaited_posts {
                tx.send(post).await.unwrap();
            }
        }
    }
}

impl Curator for RedditCurator {
    fn receiver(&mut self) -> &mut Receiver<Post> {
        &mut self.rcv
    }
}
