use std::fmt::Formatter;
use std::hash::Hasher;

use diesel::prelude::*;

use crate::schema::subscribed_listings;

#[derive(Queryable, Debug, Clone, Eq)]
pub struct Post {
    pub id: String,
    pub link: String,
    pub media_href: String,
    pub title: String,
    pub author: String,
    pub ups: i32,
    pub downs: i32,
}

impl Post {
    pub fn new(
        id: String,
        link: String,
        media_href: String,
        author: String,
        title: String,
        vote_count: (i32, i32),
    ) -> Self {
        Post {
            id,
            link,
            media_href,
            title,
            author,
            ups: vote_count.0,
            downs: vote_count.1,
        }
    }

    pub fn empty() -> Self {
        Post {
            id: String::new(),
            link: String::new(),
            media_href: String::new(),
            title: String::new(),
            author: String::new(),
            ups: 0,
            downs: 0,
        }
    }

    pub fn id(&self) -> &String {
        &self.id
    }

    pub fn title(&self) -> String {
        self.title.to_string()
    }
}

impl PartialEq for Post {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl std::hash::Hash for Post {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        state.write(self.id.as_bytes());
        state.finish();
    }
}

impl std::fmt::Display for Post {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("Post {}", self.id).as_str())
    }
}

#[derive(Queryable, Debug)]
pub struct SubscribedListing {
    pub user_id: i64,
    pub subreddit: String,
    pub category: String,
    pub head_post_id: Option<String>,
}

#[derive(Insertable, Debug)]
#[diesel(table_name = subscribed_listings)]
pub struct NewlySubscribedListing {
    pub user_id: i64,
    pub subreddit: String,
    pub category: String,
    pub head_post_id: Option<String>,
}
