use std::vec;
use std::collections::HashMap;

use reqwest::{Client, Response};
use serde_json::Value;

use crate::content::{Post, VoteCount};

pub const REDDIT_USER_AGENT: &str = "windows:com.example.artbutler:v0.0.1 (by /u/mcctor)";

pub struct Reddit;

impl Reddit {
    pub async fn retrieve_posts(
        cli: &Client,
        subreddit: &Subreddit,
        listing: &mut Listing,
    ) -> Vec<Post> {
        match Self::request_listing(cli, subreddit, &listing).await {
            Ok(resp) => {
                let result_count = listing.result_limit();
                let posts = Self::serialize(resp, result_count).await;
                if posts.len() != 0 {
                    listing.update_paginator(&posts);
                }
                posts
            }
            Err(_) => {
                vec![]
            }
        }
    }

    async fn request_listing(
        cli: &Client,
        subreddit: &Subreddit,
        listing: &Listing,
    ) -> reqwest::Result<Response> {
        cli.get(listing.endpoint_for(subreddit))
            .header("User-Agent", REDDIT_USER_AGENT)
            .send()
            .await
    }

    async fn serialize(resp: Response, result_count: u8) -> Vec<Post> {
        let raw_json = match resp.json::<Value>().await {
            Ok(v) => v,
            Err(_) => return vec![]
        };

        let mut posts = Vec::new();

        for i in 0..result_count {
            let post_raw = &raw_json["data"]["children"][i as usize]["data"];
            let post = Reddit::parse_post(&post_raw);
            if *post.id() != "ul".to_string() {
                posts.push(post);
            }
        }

        return posts;
    }

    fn parse_post(raw_json: &Value) -> Post {
        let mut fields = HashMap::new();
        {
            fields.insert("id".to_string(), raw_json["id"].to_string());
            fields.insert("url".to_string(), raw_json["url"].to_string());
            fields.insert("author".to_string(), raw_json["author"].to_string());
            fields.insert("title".to_string(), raw_json["title"].to_string());
            Reddit::normalize(&mut fields);
        }

        let ups = match raw_json["ups"].to_string().parse() {
            Ok(value) => value,
            Err(_) => 0,
        };
        let downs = match raw_json["downs"].to_string().parse() {
            Ok(value) => value,
            Err(_) => 0,
        };

        Post::new(
            fields.remove("id").unwrap(),
            fields.remove("url").unwrap(),
            fields.remove("author").unwrap(),
            fields.remove("title").unwrap(),
            VoteCount::from(ups, downs),
            raw_json["num_comments"].to_string().parse().unwrap_or(0),
        )
    }

    fn normalize(elems: &mut HashMap<String, String>) {
        for (_, value) in elems.iter_mut() {
            value.remove(0);
            value.remove(value.len() - 1);
        }
    }
}

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct Subreddit(String);

impl Subreddit {
    pub fn from(name: &str) -> Self {
        Subreddit(name.to_string())
    }

    pub fn name(&self) -> String {
        self.0.to_string()
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum Time {
    Hour,
    Day,
    Week,
    Month,
    Year,
    All,
}

#[derive(PartialEq, Debug, Clone)]
pub enum Seek {
    After { post_id: String },
    Before { post_id: String },
}

#[derive(PartialEq, Debug, Clone)]
pub struct PaginationArg {
    pub cursor_anchor: Seek,
    pub limit: u8,
    pub seen_count: u8,
    pub show_rules: String,
}

impl PaginationArg {
    pub fn default() -> Self {
        PaginationArg {
            cursor_anchor: Seek::After {
                post_id: "null".to_string(),
            },
            limit: 5,
            seen_count: 0,
            show_rules: "null".to_string(),
        }
    }

    pub fn seek_forward(&mut self, cursor: Option<String>) {
        if let Some(post_id) = cursor {
            self.cursor_anchor = Seek::After { post_id };
        } else {
            self.cursor_anchor = Seek::After {
                post_id: "null".to_string(),
            };
        }
    }

    pub fn seek_backward(&mut self, cursor: Option<String>) {
        if let Some(post_id) = cursor {
            self.cursor_anchor = Seek::Before { post_id };
        } else {
            self.cursor_anchor = Seek::Before {
                post_id: "null".to_string(),
            };
        }
    }

    pub fn set_res_limit(&mut self, limit: u8) -> &mut Self {
        self.limit = limit;
        self
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum Listing {
    Hot { params: PaginationArg },
    New { params: PaginationArg },
    Rising { params: PaginationArg },
    Sort { params: PaginationArg, time: Time },
    Random,
}

impl Listing {
    pub fn endpoint_for(&self, subreddit: &Subreddit) -> String {
        let mut href_buf = String::new();
        href_buf.push_str(format!("https://api.reddit.com/r/{}/", subreddit.name()).as_str());

        match self {
            Self::Hot { params } => {
                href_buf.push_str(self.listing_tag());
                href_buf.push_str(self.url_args(params).as_str());
            }
            Self::New { params } => {
                href_buf.push_str(self.listing_tag());
                href_buf.push_str(self.url_args(params).as_str());
            }
            Self::Rising { params } => {
                href_buf.push_str(self.listing_tag());
                href_buf.push_str(self.url_args(params).as_str());
            }
            Self::Sort { time, params } => {
                href_buf.push_str(self.listing_tag());
                href_buf.push_str(self.url_args(params).as_str());
            }
            Self::Random => {
                href_buf.push_str(self.listing_tag());
            }
        };

        href_buf
    }

    pub fn update_paginator(&mut self, with: &Vec<Post>) {
        match self {
            Listing::Random => (),
            listing => {
                let paginator = listing.paginator();
                match paginator {
                    PaginationArg { cursor_anchor, .. } => match cursor_anchor {
                        Seek::After { post_id } => {
                            let new_id = with.last().unwrap().id().to_string();
                            if new_id != "ul".to_string() {
                                *post_id = format!("t3_{}", with.last().unwrap().id().to_string())
                            }
                        }
                        Seek::Before { post_id } => {
                            *post_id = format!("t3_{}", with.first().unwrap().id().to_string())
                        }
                    },
                }
            }
        };
    }

    pub fn result_limit(&mut self) -> u8 {
        match self {
            Listing::Random => 1,
            listing => listing.paginator().limit,
        }
    }

    pub fn paginator(&mut self) -> &mut PaginationArg {
        match self {
            Self::Hot { params } => params,
            Self::New { params } => params,
            Self::Rising { params } => params,
            Self::Sort { params, .. } => params,
            Self::Random => panic!("pagination does not exist for `SubredditListing::Random`"),
        }
    }

    fn listing_tag(&self) -> &'static str {
        match self {
            Self::Hot { .. } => "hot",
            Self::New { .. } => "new",
            Self::Random { .. } => "random",
            Self::Rising { .. } => "rising",
            Self::Sort { .. } => "sort",
        }
    }

    fn url_args(&self, pagination_arg: &PaginationArg) -> String {
        let mut buf = String::new();
        match &pagination_arg.cursor_anchor {
            Seek::After { post_id: id } => {
                let arg = format!("?after={}", id);
                buf.push_str(arg.as_str())
            }
            Seek::Before { post_id: id } => {
                let arg = format!("?before={}", id);
                buf.push_str(arg.as_str())
            }
        };
        buf.push_str(
            format!(
                "&count={}&limit={}&show={}",
                pagination_arg.seen_count, pagination_arg.limit, pagination_arg.show_rules
            ).as_str(),
        );
        buf
    }
}

impl ToString for Listing {
    fn to_string(&self) -> String {
        self.listing_tag().to_string()
    }
}