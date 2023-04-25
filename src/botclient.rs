use std::env;

use diesel::prelude::*;
use diesel::result::DatabaseErrorKind::UniqueViolation;
use diesel::result::Error;
use dotenvy::dotenv;
use log::{error, info};

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

#[derive(Insertable)]
#[diesel(table_name = botclients)]
struct NewClient {
    id: i64,
    username: Option<String>,
    is_user: bool,
}

impl PartialEq for BotClient {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub struct ClientManager {
    db: PgConnection,
    existing: Vec<BotClient>,
}

impl ClientManager {
    pub fn instance() -> Self {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let db = PgConnection::establish(&database_url)
            .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

        Self {
            db,
            existing: vec![],
        }
    }

    pub fn add_new_user(
        &mut self,
        id: ClientID,
        username: Option<String>,
        is_user: bool,
    ) -> BotClient {
        let cli = BotClient {
            id,
            username,
            is_user,
        };
        self.save_to_db(&cli);
        self.existing.push(cli.clone());
        cli
    }

    pub fn get(&mut self, user: ClientID) -> Option<BotClient> {
        use crate::botclient::*;
        use crate::schema::botclients::dsl::*;

        let client = botclients.find(user.0).get_result(&mut self.db);
        if let Ok(cli) = client {
            self.existing.push(cli);
            let end = self.existing.len() - 1;
            return Some(self.existing.get(end).unwrap().clone());
        }
        None
    }

    fn save_to_db(&mut self, new_user: &BotClient) {
        let username = match new_user.username.clone() {
            None => None,
            Some(username) => Some(username),
        };
        let new_client = NewClient {
            id: new_user.id.id(),
            username,
            is_user: new_user.is_user,
        };

        let res = diesel::insert_into(botclients::table)
            .values(&new_client)
            .execute(&mut self.db);

        if let Ok(v) = res {
            info!("Registered and added new {:?} to database", new_user);
        } else {
            if let Some(Error::DatabaseError(kind, _)) = res.err() {
                if let UniqueViolation = kind {
                    error!(
                        "Attempting to register the same client twice {:?}",
                        new_user
                    )
                }
            }
        }
    }
}
