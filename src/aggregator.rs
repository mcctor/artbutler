use std::collections::VecDeque;
use std::env;
use std::sync::Arc;

use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenvy::dotenv;
use reqwest::ClientBuilder;

use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;

use crate::content::{Client, ClientID, Post};
use crate::curator::Curator;
use crate::listings::reddit::{Api, Listing};
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

pub struct UserAggregator<SRC> {
    id: ClientID,
    pub curator: Option<Curator<SRC>>,
    listings: VecDeque<Listing>,
    cache: VecDeque<Post>,
    pub chan: (Sender<Post>, Receiver<Post>),
}

impl<SRC> UserAggregator<SRC>
where
    SRC: ListingSource + Send + Sync + 'static,
{
    fn new(id: ClientID) -> Self {
        let (tx, rcv) = channel(10);
        UserAggregator {
            id,
            listings: Default::default(),
            cache: VecDeque::with_capacity(5),
            chan: (tx, rcv),
            curator: None,
        }
    }

    pub fn add_listing(&mut self, category: Listing) {
        if self.curator.is_none() {
            panic!("must attach a Curator to an UserAggregator before calling UserAggregator::add_listing")
        }
        let listing = Arc::new(Mutex::new(category));
        self.curator.as_mut().unwrap().spawn_for(listing);
    }

    pub fn attach_curator(&mut self, curator: Curator<SRC>) {
        self.curator = Some(curator);
    }
}

pub struct AggregatorStore {
    db: PgConnection,
}

impl AggregatorStore {
    pub fn instance() -> Self {
        use crate::content::*;
        use crate::schema::botclients::dsl::*;

        // let mut db = Self::db_instance();
        // let existing_clients = botclients.load::<Client>(&mut db);
        //
        Self {
            db: Self::db_instance(),
        }
    }

    fn db_instance() -> PgConnection {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        PgConnection::establish(&database_url)
            .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
    }

    pub fn find(&mut self, client: ClientID) -> Option<UserAggregator<Api>> {
        use crate::content::*;
        use crate::schema::subscribed_listings::dsl::*;

        let listings = subscribed_listings
            .filter(user_id.eq(client.id()))
            .load::<SubscribedListing>(&mut self.db)
            .expect("error loading subscribed listings.");

        let client_req = ClientBuilder::new()
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap();

        let mut aggregator: UserAggregator<Api> = UserAggregator::new(client);
        aggregator.attach_curator(Curator::from(Api::from(&client_req)));
        for listing in listings {
            let listing = Listing::from(listing.category.as_str(), listing.subreddit.into());
            aggregator.add_listing(listing);
        }

        Some(aggregator)
    }

    pub fn create(&mut self, client_id: ClientID) -> &mut UserAggregator<Api> {
        todo!()
    }
}
