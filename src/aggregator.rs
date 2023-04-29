use std::collections::VecDeque;
use std::env;
use std::sync::Arc;
use std::time::Duration;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenvy::dotenv;
use tokio::spawn;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::{Mutex, MutexGuard};
use tokio::time::{sleep_until, Instant};

use crate::botclient::ClientID;
use crate::content::{NewlySubscribedListing, Post};
use crate::curator::Curator;
use crate::listings::reddit::{Api, Listing};
use crate::listings::source::ListingSource;
use crate::schema::subscribed_listings;

pub struct UserAggregator<SRC> {
    id: ClientID,
    curator: Curator<SRC>,
    pub listings: VecDeque<Arc<Mutex<Listing>>>,
    pub cache: Arc<Mutex<VecDeque<Post>>>,
    db: PgConnection,
}

impl<SRC> UserAggregator<SRC>
where
    SRC: ListingSource,
{
    fn new(id: ClientID) -> Arc<Mutex<UserAggregator<SRC>>> {
        let aggr = UserAggregator {
            id,
            listings: Default::default(),
            cache: Arc::new(Mutex::new(VecDeque::with_capacity(5))),
            db: Self::db_instance(),
            curator: Curator::from(SRC::default()),
        };
        let aggr = Arc::new(Mutex::new(aggr));
        spawn(Self::listen(aggr.clone()));
        aggr
    }

    pub async fn latest(&mut self) -> Vec<Post> {
        let mut cache_guard = self.cache.lock().await;
        let mut buf = vec![];

        let cache_len = cache_guard.len();
        let mut index = 0;
        while index < cache_len {
            buf.push(cache_guard.pop_front().unwrap().clone());
            index += 1;
        }
        buf
    }

    async fn listen(aggr: Arc<Mutex<UserAggregator<SRC>>>) {
        // TODO: Here is where you at.
        loop {
            let mut aggr = aggr.lock().await;
            if let Some(post) = aggr.curator.chan.1.recv().await {
                let mut cache_guard = aggr.cache.lock().await;
                cache_guard.push_back(post);
            } else {
                break;
            }
            sleep_until(Instant::now() + Duration::from_secs(2)).await;
        }
    }

    fn db_instance() -> PgConnection {
        dotenv().ok();

        let db_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set.");
        PgConnection::establish(&db_url)
            .unwrap_or_else(|_| panic!("Error connecting to {}", db_url))
    }

    pub fn save_to_db(&mut self, new_listing: &Listing) -> QueryResult<usize> {
        let listing = NewlySubscribedListing {
            user_id: self.id.id(),
            subreddit: new_listing.subreddit().name(),
            category: new_listing.tag().to_string(),
            head_post_id: None,
        };
        diesel::insert_into(subscribed_listings::table)
            .values(listing)
            .execute(&mut self.db)
    }

    pub fn add_listing(&mut self, category: Arc<Mutex<Listing>>) {
        self.listings.push_back(category.clone());
        self.curator.spawn_for(category);
    }
}

pub struct AggregatorStore {
    db: PgConnection,
}

impl Clone for AggregatorStore {
    fn clone(&self) -> Self {
        Self {
            db: Self::db_instance(),
        }
    }
}

impl AggregatorStore {
    pub fn instance() -> Self {
        Self {
            db: Self::db_instance(),
        }
    }

    fn db_instance() -> PgConnection {
        dotenv().ok().unwrap();

        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        PgConnection::establish(&database_url)
            .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
    }

    pub async fn find(&mut self, id: ClientID) -> Option<Arc<Mutex<UserAggregator<Api>>>> {
        use crate::content::*;
        use crate::schema::subscribed_listings::dsl::*;

        let listings = subscribed_listings
            .filter(user_id.eq(id.id()))
            .load::<SubscribedListing>(&mut self.db)
            .expect("error loading subscribed listings.");

        let aggregator = UserAggregator::new(id);
        for listing in listings {
            let listing = Listing::from(listing.category.as_str(), listing.subreddit.into());
            let mut aggregator = aggregator.lock().await;
            aggregator.add_listing(Arc::new(Mutex::new(listing)));
        }

        Some(aggregator)
    }
}
