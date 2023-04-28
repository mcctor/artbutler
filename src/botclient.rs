use std::env;
use std::sync::Arc;

use diesel::prelude::*;
use dotenvy::dotenv;
use log::warn;
use tokio::sync::Mutex;

use crate::aggregator::{AggregatorStore, UserAggregator};
use crate::listings::reddit::Api;
use crate::schema::botclients;

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct ClientID(i64);

impl ClientID {
    pub fn id(&self) -> i64 {
        self.0
    }
}

impl From<i64> for ClientID {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl From<ClientID> for i64 {
    fn from(value: ClientID) -> Self {
        value.0
    }
}

#[derive(Queryable, Clone, Debug)]
pub struct BotClient {
    #[diesel(deserialize_as = i64)]
    id: ClientID,
    username: Option<String>,
    is_user: bool,
}

impl BotClient {
    pub fn id(&self) -> i64 {
        self.id.0
    }
}

impl PartialEq for BotClient {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Insertable)]
#[diesel(table_name = botclients)]
struct NewClient {
    id: i64,
    username: Option<String>,
    is_user: bool,
}

pub struct ClientManager {
    db: PgConnection,
    aggr_store: AggregatorStore,
    pub existing: Vec<Arc<(BotClient, Arc<Mutex<UserAggregator<Api>>>)>>,
}

impl ClientManager {
    pub async fn instance() -> ConnectionResult<Self> {
        dotenv().ok().unwrap();

        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let db = PgConnection::establish(&database_url)?;
        let mut cli_mgr = Self {
            db,
            aggr_store: AggregatorStore::instance(),
            existing: vec![],
        };
        cli_mgr
            .load_all()
            .await
            .expect("Failed to load all clients");

        Ok(cli_mgr)
    }

    pub async fn add_new_user(
        &mut self,
        id: ClientID,
        username: Option<String>,
        is_user: bool,
    ) -> ConnectionResult<Arc<(BotClient, Arc<Mutex<UserAggregator<Api>>>)>> {
        let cli = BotClient {
            id,
            username,
            is_user,
        };
        self.flush_to_db(&cli)
            .unwrap_or_else(|_| warn!("Unable to save client to DB"));

        let aggr = self.aggr_store.find(cli.id).await.unwrap();
        let cli = Arc::new((cli, aggr));
        self.existing.push(cli.clone());
        Ok(cli)
    }

    pub async fn get(
        &mut self,
        user: ClientID,
    ) -> Option<Arc<(BotClient, Arc<Mutex<UserAggregator<Api>>>)>> {
        use crate::botclient::*;
        use crate::schema::botclients::dsl::*;

        let is_existent = self.existing.iter_mut().find(|value| {
            let cli = &value.0;
            cli.id == user
        });
        if let Some(..) = is_existent {
            Some(is_existent.unwrap().clone())
        } else {
            let cli_result = botclients
                .find(user.id())
                .get_result::<BotClient>(&mut self.db);

            if let Ok(cli) = cli_result {
                let aggr = self.aggr_store.find(cli.id).await.unwrap();
                let cli = Arc::new((cli, aggr));
                self.existing.push(cli);
                let end = self.existing.len() - 1;
                return Some(self.existing.get(end).unwrap().clone());
            }
            None
        }
    }

    async fn load_all(&mut self) -> QueryResult<()> {
        use crate::botclient::*;
        use crate::schema::botclients::dsl::*;

        self.existing = {
            let mut buf = vec![];
            for client in botclients.load::<BotClient>(&mut self.db)? {
                let aggr = self.aggr_store.find(client.id).await.unwrap();
                buf.push(Arc::new((client, aggr)));
            }
            buf
        };
        Ok(())
    }

    fn flush_to_db(&mut self, new_client: &BotClient) -> QueryResult<()> {
        let username = new_client.username.clone();
        let cli_row = NewClient {
            id: new_client.id.id(),
            username,
            is_user: new_client.is_user,
        };

        diesel::insert_into(botclients::table)
            .values(&cli_row)
            .execute(&mut self.db)?;

        Ok(())
    }
}
