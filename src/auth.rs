use std::env;

use diesel::prelude::*;
use diesel::result::DatabaseErrorKind::UniqueViolation;
use diesel::result::Error;
use dotenvy::dotenv;
use log::warn;
use teloxide::types::UserId;

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
    pub id: ClientID,

    pub username: Option<String>,
    pub is_user: bool,
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

    pub fn get(&mut self, user: ClientID) -> Option<&BotClient> {
        use crate::auth::*;
        use crate::schema::botclients::dsl::*;

        let client = botclients.find(user.0).get_result(&mut self.db);
        if let Ok(cli) = client {
            self.existing.push(cli);
            let end = self.existing.len() - 1;
            return Some(self.existing.get(end).unwrap());
        }
        None
    }

    pub fn add(&mut self, new_user: BotClient) {
        let username = new_user.username;
        let new_client = NewClient {
            id: new_user.id.id(),
            username,
            is_user: new_user.is_user,
        };

        let res = diesel::insert_into(botclients::table)
            .values(&new_client)
            .execute(&mut self.db);
        if res.is_err() {
            match res.err().unwrap() {
                Error::DatabaseError(kind, info) => match kind {
                    UniqueViolation => {
                        warn!(
                            "Registering the same client twice ClientID \"{}\" ",
                            new_client.id
                        );
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

#[test]
fn test_client_manager() {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let conn = PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

    let username = Some("Vanessa".to_string());
    let mut client_manager = ClientManager::instance();
    client_manager.add(BotClient {
        id: ClientID(89999222654),
        username: username.clone(),
        is_user: true,
    });

    let vannessa = client_manager.get(89999222654.into()).unwrap();
    assert_eq!(username, vannessa.username);
    println!("{:?}", vannessa);
}
