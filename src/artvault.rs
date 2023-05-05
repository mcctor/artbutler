use std::env;

use diesel::prelude::*;
use diesel::result::Error;
use diesel::PgConnection;
use dotenvy::dotenv;
use log::warn;

use crate::content::{NewPost, Post};
use crate::schema::artposts;
use crate::schema::artposts::dsl::*;

pub struct ArtVault {
    db: PgConnection,
}

impl ArtVault {
    pub fn instance() -> Self {
        Self {
            db: Self::db_instance(),
        }
    }

    pub fn save(&mut self, p: &Post) {
        let new_post = NewPost {
            id: p.id.to_string(),
            media_href: p.media_href.to_string(),
            title: p.title.to_string(),
            author: p.author.to_string(),
            ups: p.ups,
            downs: p.downs,
        };

        let res = diesel::insert_into(artposts::table)
            .values(&new_post)
            .execute(&mut self.db);
        if res.is_err() {
            match res.err().unwrap() {
                Error::DatabaseError(kind, info) => match kind {
                    diesel::result::DatabaseErrorKind::UniqueViolation => {
                        warn!("Trying to same the same PostID \"{}\" ", p.id.to_string());
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }

    pub fn fetch(&mut self, post_id: &str) -> Option<Post> {
        let found_post = artposts.find(post_id).get_result(&mut self.db);

        if found_post.is_ok() {
            Some(found_post.unwrap())
        } else {
            None
        }
    }

    fn db_instance() -> PgConnection {
        dotenv().ok();
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        PgConnection::establish(&database_url)
            .unwrap_or_else(|_| panic!("Error connecting to {}", database_url))
    }
}
