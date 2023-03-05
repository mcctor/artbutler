use std::collections::HashMap;
use std::time::Duration;
use std::{env, vec};
use dotenvy::dotenv;

use reqwest::{Result, Client, Response};
use serde_json::Value;
use tokio::time::Instant;

use crate::content::{Post, VoteCount};

pub const REDDIT_USER_AGENT: &str = "windows:com.example.artbutler:v0.0.1 (by /u/mcctor)";

enum AuthenticationToken {
    New,
    Refresh,
}

#[derive(Debug, Clone)]
pub struct BearerToken {
    token: String,
    expires: Instant,
}

#[derive(Debug)]
pub struct Reddit {
    cli: Client,
    has_token: Option<BearerToken>,
}

impl Reddit {
    pub fn new() -> Self {
        Reddit { cli: Client::new(), has_token: None }
    }

    async fn authenticate(&mut self, auth_option: AuthenticationToken) -> Result<&BearerToken> {
        dotenv().expect("no .env file found");

        let client_id = env::var("CLIENT_ID").expect("CLIENT_ID not provided");
        let secret = env::var("SECRET").expect("SECRET not provided");
        let username = env::var("USER_NAME").expect("USERNAME not provided");
        let pass = env::var("PASSWORD").expect("PASSWORD not provided");

        let mut args = String::new();
        match auth_option {
            AuthenticationToken::New => {
                let params = format!(
                    "?grant_type=password&username={}&password={}",
                    username,
                    pass
                );
                args.push_str(params.as_str());
            }
            AuthenticationToken::Refresh => {
                let params = format!(
                    "?grant_type=refresh_token&refresh_token={}",
                    self.has_token.as_ref().unwrap().token
                );
                args.push_str(params.as_str());
            }
        }
        let url = format!("https://www.reddit.com/api/v1/access_token{}", args);
        let res = self.cli.post(url)
            .basic_auth(client_id, Some(secret))
            .header("User-Agent", REDDIT_USER_AGENT)
            .send().await?;

        let value = res.json::<Value>().await?;
        let token = Some(BearerToken {
            token: value["access_token"].as_str().unwrap().to_string(),
            expires: Instant::now() + Duration::from_secs(value["expires_in"].as_u64().unwrap()),
        });
        self.has_token = token;
        Ok(self.has_token.as_ref().unwrap())
    }

    pub async fn retrieve_posts(&mut self, sub: &Subreddit, listing: &mut Listing) -> Result<Vec<Post>> {
        let resp = self.request_listing(sub, listing).await?;
        let res_cnt = listing.result_limit();
        let posts = Reddit::serialize(resp, res_cnt).await;
        if posts.len() != 0 {
            listing.update_paginator(&posts);
        }
        Ok(posts)
    }

    async fn request_listing(&mut self, sub: &Subreddit, listing: &Listing) -> Result<Response> {
        let req_builder = self.cli.get(listing.endpoint_for(sub));
        if let Some(t) = self.has_token.as_ref() {
            if Instant::now() > (t.expires - Duration::from_secs(60)) {
                self.authenticate(AuthenticationToken::Refresh).await?;
            }
        } else {
            self.authenticate(AuthenticationToken::New).await?;
        }

        let res = req_builder
            .bearer_auth(self.has_token.as_ref().unwrap().token.to_string())
            .header("User-Agent", REDDIT_USER_AGENT)
            .send().await?;

        Ok(res)
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

        posts
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

        let post_url = {
            let perma = raw_json["permalink"].as_str().unwrap_or("");
            if perma.is_empty() {
                "".to_string()
            } else {
                format!("https://www.reddit.com{}", perma)
            }
        };

        Post::new(
            fields.remove("id").unwrap(),
            post_url,
            fields.remove("url").unwrap(),
            fields.remove("author").unwrap(),
            fields.remove("title").unwrap(),
            VoteCount::from(ups, downs),
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
    After { post: Post },
    Before { post: Post },
}

#[derive(PartialEq, Debug, Clone)]
pub struct PaginationArg {
    pub cursor_anchor: Seek,
    pub limit: u8,
    pub seen_count: u8,
    pub show_rules: String,
}

impl PaginationArg {
    pub fn seek_back() -> Self {
        PaginationArg {
            cursor_anchor: Seek::After {
                post: Post::empty(),
            },
            limit: 5,
            seen_count: 0,
            show_rules: "null".to_string(),
        }
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
        href_buf.push_str(format!("https://oauth.reddit.com/r/{}/", subreddit.name()).as_str());

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
                match listing.paginator() {
                    PaginationArg { cursor_anchor, .. } => match cursor_anchor {
                        Seek::After { post } => {
                            let new_id = with.last().unwrap().id().to_string();
                            if new_id != "ul".to_string() {
                                *post = with.last().unwrap().clone();
                            }
                        }
                        Seek::Before { post } => {
                            *post = with.first().unwrap().clone();
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
            Seek::After { post } => {
                let arg = format!("?after={}", format!("t3_{}", post.id()));
                buf.push_str(arg.as_str())
            }
            Seek::Before { post } => {
                let arg = format!("?before={}", format!("t3_{}", post.id()));
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