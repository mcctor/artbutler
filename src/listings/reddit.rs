use std::collections::{HashMap, VecDeque};
use std::env;
use std::time::Duration;

use async_trait::async_trait;
use dotenvy::dotenv;
use log::info;

use reqwest::{Client, Response, Result};
use serde_json::Value;
use tokio::time::Instant;

use crate::content::{Post, VoteCount};
use crate::listings::reddit::Listing::{Hot, New, Random, Rising, Sort};
use crate::listings::source::ListingSource;

const REDDIT_USER_AGENT: &str = "windows:com.example.artbutler:v0.0.1 (by /u/mcctor)";

enum AuthTokenAction {
    New,
    Refresh,
}

#[derive(Debug, Clone)]
pub struct BearerToken {
    token: String,
    expires: Instant,
}

#[derive(Debug, Clone)]
pub struct Api {
    cli: Client,
    has_token: Option<BearerToken>,
}

#[tokio::test]
async fn test_request() {
    let cli = Client::new();
    let mut api = Api::from(&cli);

    let mut pagination = Pagination::builder();
    pagination.set_cursor(Post::empty());
    pagination.set_limit(2);
    pagination.seek_back();

    let mut new_listing = New {
        subreddit: Subreddit("earthporn".to_string()),
        paginator: pagination,
    };
    let res = api.request(&new_listing).await.unwrap();

    let value = res.json::<Value>().await.unwrap();
    let parsed_children = value["data"]["children"].as_array().unwrap();
    assert_eq!(new_listing.result_limit() as usize, parsed_children.len());
    println!("{:#?}", parsed_children);
}

#[tokio::test]
async fn test_multi_request_seek_back() {
    let cli = Client::new();
    let mut api = Api::from(&cli);

    let mut pagination = Pagination::builder();
    pagination.set_cursor(Post::empty());
    pagination.set_limit(2);
    pagination.seek_forward();

    let mut new_listing = New {
        subreddit: Subreddit("artbutler".to_string()),
        paginator: pagination,
    };
    let first_res = api.request(&new_listing).await.unwrap();
    let batch_a = api
        .serialize(first_res, new_listing.result_limit())
        .await
        .unwrap();
    if !batch_a.is_empty() {
        new_listing.update_paginator_cache(&batch_a);
    }

    let second_res = api.request(&new_listing).await.unwrap();
    let batch_b = api
        .serialize(second_res, new_listing.result_limit())
        .await
        .unwrap();
    if !batch_b.is_empty() {
        new_listing.update_paginator_cache(&batch_b);
    }

    println!("{:#?}", batch_a);
    println!("{:#?}", batch_b);
}

#[tokio::test]
async fn test_serialize() {
    let cli = Client::new();
    let mut api = Api::from(&cli);

    let mut pagination = Pagination::builder();
    pagination.set_cursor(Post::empty());
    pagination.set_limit(4);

    let mut new_listing = New {
        subreddit: Subreddit("artbutler".to_string()),
        paginator: pagination,
    };

    let res = api.request(&new_listing).await.unwrap();
    let parsed = api
        .serialize(res, new_listing.result_limit())
        .await
        .unwrap();
    assert_eq!(parsed.len(), new_listing.result_limit() as usize);

    let mut cnt = new_listing.result_limit();
    for post in &parsed {
        let post_title = format!("Post {}", cnt);
        assert_eq!(post.title(), post_title);
        cnt -= 1;
    }
    println!("{:#?}", parsed);
}

#[async_trait]
impl ListingSource for Api {
    async fn retrieve_posts(&mut self, listing: &mut Listing) -> Result<VecDeque<Post>> {
        let resp = self.request(listing).await?;
        let posts = self.serialize(resp, listing.result_limit()).await.unwrap();
        if !posts.is_empty() {
            listing.update_paginator_cache(&posts);
        }

        Ok(posts)
    }
}

impl Api {
    pub fn from(cli: &Client) -> Self {
        dotenv().ok().unwrap();
        Api {
            cli: cli.clone(),
            has_token: None,
        }
    }

    async fn authenticate(&mut self, auth_option: AuthTokenAction) -> Result<&BearerToken> {
        let client_id = env::var("CLIENT_ID").expect("CLIENT_ID not provided");
        let secret = env::var("SECRET").expect("SECRET not provided");
        let username = env::var("USER_NAME").expect("USERNAME not provided");
        let pass = env::var("PASSWORD").expect("PASSWORD not provided");

        let args = Api::auth_url_args(username, pass, self.has_token.as_ref(), auth_option);
        let url = format!("https://www.reddit.com/api/v1/access_token{}", args);
        let res = self
            .cli
            .post(url)
            .basic_auth(client_id, Some(secret))
            .header("User-Agent", REDDIT_USER_AGENT)
            .send()
            .await?;

        let value = res.json::<Value>().await?;
        let token = Some(BearerToken {
            token: value["access_token"].as_str().unwrap().to_string(),
            expires: Instant::now() + Duration::from_secs(value["expires_in"].as_u64().unwrap()),
        });

        info!("Reddit API is authenticated");
        self.has_token = token;
        Ok(self.has_token.as_ref().unwrap())
    }

    fn auth_url_args(
        username: String,
        pass: String,
        token: Option<&BearerToken>,
        auth_option: AuthTokenAction,
    ) -> String {
        let mut args = String::new();
        match auth_option {
            AuthTokenAction::New => {
                let params = format!(
                    "?grant_type=password&username={}&password={}",
                    username, pass
                );
                args.push_str(params.as_str());
            }
            AuthTokenAction::Refresh => {
                let params = format!(
                    "?grant_type=refresh_token&refresh_token={}",
                    token.as_ref().unwrap().token
                );
                args.push_str(params.as_str());
            }
        };
        args
    }

    pub async fn authenticate_or_refresh(&mut self) -> Result<&BearerToken> {
        if let Some(t) = self.has_token.as_ref() {
            if Instant::now() > (t.expires - Duration::from_secs(60)) {
                info!("Reddit API bearer token refreshed");
                self.authenticate(AuthTokenAction::Refresh).await?;
            }
            Ok(self.has_token.as_ref().unwrap())
        } else {
            self.authenticate(AuthTokenAction::New).await
        }
    }

    async fn request(&mut self, listing: &Listing) -> Result<Response> {
        let req_builder = self.cli.get(listing.endpoint());
        let bearer = self.authenticate_or_refresh().await?;
        let res = req_builder
            .bearer_auth(bearer.token.to_string())
            .header("User-Agent", REDDIT_USER_AGENT)
            .send()
            .await?;

        Ok(res)
    }

    async fn serialize(&self, resp: Response, result_count: u64) -> Result<VecDeque<Post>> {
        let raw_json = resp.json::<Value>().await?;
        let mut posts = VecDeque::new();
        for i in 0..result_count {
            let post_raw = &raw_json["data"]["children"][i as usize]["data"];
            let post = self.parse_post(&post_raw);
            if *post.id() != "ul".to_string() {
                posts.push_front(post);
            }
        }

        Ok(posts)
    }

    fn parse_post(&self, raw_json: &Value) -> Post {
        let mut fields = HashMap::new();
        {
            fields.insert("id".to_string(), raw_json["id"].to_string());
            fields.insert("url".to_string(), raw_json["url"].to_string());
            fields.insert("author".to_string(), raw_json["author"].to_string());
            fields.insert("title".to_string(), raw_json["title"].to_string());
            Api::normalize(&mut fields);
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
    pub fn name(&self) -> String {
        self.0.to_string()
    }
}

impl<T: ToString> From<T> for Subreddit {
    fn from(s: T) -> Self {
        Self(s.to_string())
    }
}

#[derive(PartialEq, Debug, Copy, Clone, Eq, Hash)]
pub enum Time {
    Hour,
    Day,
    Week,
    Month,
    Year,
    All,
}

#[derive(PartialEq, Debug, Clone, Eq, Hash)]
pub enum Seek {
    After { cache: VecDeque<Post> },
    Back { cache: VecDeque<Post> },
}

#[derive(PartialEq, Debug, Clone, Eq, Hash)]
pub struct Pagination {
    cursor_anchor: Seek,
    limit: u64,
    seen_count: u64,
    show_rules: String,
}

impl Pagination {
    pub fn builder() -> Self {
        Pagination {
            cursor_anchor: Seek::After {
                cache: Default::default(),
            },
            limit: 0,
            seen_count: 0,
            show_rules: "".to_string(),
        }
    }

    pub fn cursor(&self) -> &Seek {
        &self.cursor_anchor
    }

    pub fn set_cursor(&mut self, p: Post) -> &mut Self {
        match &mut self.cursor_anchor {
            Seek::After { cache } => {
                cache.push_back(p);
            }
            Seek::Back { cache } => {
                cache.push_back(p);
            }
        };
        self
    }

    pub fn set_limit(&mut self, value: u64) -> &mut Self {
        self.limit = value;
        self
    }

    pub fn seek_forward(&mut self) {
        self.cursor_anchor = Seek::After {
            cache: Default::default(),
        }
    }

    pub fn seek_back(&mut self) {
        self.cursor_anchor = Seek::Back {
            cache: Default::default(),
        }
    }
}

#[derive(PartialEq, Debug, Clone, Eq, Hash)]
pub enum Listing {
    Hot {
        subreddit: Subreddit,
        paginator: Pagination,
    },
    New {
        subreddit: Subreddit,
        paginator: Pagination,
    },
    Rising {
        subreddit: Subreddit,
        paginator: Pagination,
    },
    Sort {
        subreddit: Subreddit,
        paginator: Pagination,
        time: Time,
    },
    Random {
        subreddit: Subreddit,
    },
}

impl Listing {
    pub fn from(listing_name: &str, sub: Subreddit) -> Listing {
        let mut pagination = Pagination::builder();
        pagination.set_cursor(Post::empty());
        pagination.set_limit(5);

        match listing_name {
            "hot" => Hot {
                subreddit: sub,
                paginator: pagination.clone(),
            },
            "new" => New {
                subreddit: sub,
                paginator: pagination.clone(),
            },
            "rising" => Rising {
                subreddit: sub,
                paginator: pagination.clone(),
            },
            "sort" => Sort {
                subreddit: sub,
                paginator: pagination.clone(),
                time: Time::Hour,
            },
            "random" => Random { subreddit: sub },
            _ => panic!("invalid listing name"),
        }
    }
    pub fn endpoint(&self) -> String {
        let mut href_buf = String::new();
        href_buf.push_str("https://oauth.reddit.com");

        match self {
            Random { subreddit } => {
                href_buf.push_str(format!("/r/{}/", subreddit.name()).as_str());
                href_buf.push_str(self.listing_tag());
            }
            Sort {
                subreddit,
                time,
                paginator: params,
            } => {
                href_buf.push_str(format!("/r/{}/", subreddit.name()).as_str());
                href_buf.push_str(self.listing_tag());
                href_buf.push_str(self.url_args(params).as_str());
            }
            Hot {
                subreddit,
                paginator: params,
            } => {
                href_buf.push_str(format!("/r/{}/", subreddit.name()).as_str());
                href_buf.push_str(self.listing_tag());
                href_buf.push_str(self.url_args(params).as_str());
            }
            New {
                subreddit,
                paginator: params,
            } => {
                href_buf.push_str(format!("/r/{}/", subreddit.name()).as_str());
                href_buf.push_str(self.listing_tag());
                href_buf.push_str(self.url_args(params).as_str());
            }
            Rising {
                subreddit,
                paginator: params,
            } => {
                href_buf.push_str(format!("/r/{}/", subreddit.name()).as_str());
                href_buf.push_str(self.listing_tag());
                href_buf.push_str(self.url_args(params).as_str());
            }
        };
        href_buf
    }

    pub fn subreddit(&self) -> Subreddit {
        match self {
            Hot { subreddit, .. } => subreddit.clone(),
            New { subreddit, .. } => subreddit.clone(),
            Rising { subreddit, .. } => subreddit.clone(),
            Sort { subreddit, .. } => subreddit.clone(),
            Random { subreddit } => subreddit.clone(),
        }
    }

    pub fn update_paginator_cache(&mut self, with: &VecDeque<Post>) {
        match self {
            Random { .. } => (),
            listing => match &mut listing.paginator().cursor_anchor {
                Seek::After { cache: posts } => {
                    posts.clear();
                    for post in with {
                        posts.push_back(post.clone());
                    }
                }
                Seek::Back { cache: posts } => {
                    posts.clear();
                    for post in with {
                        posts.push_front(post.clone());
                    }
                }
            },
        };
    }

    pub fn result_limit(&mut self) -> u64 {
        match self {
            Random { .. } => 1,
            listing => listing.paginator().limit,
        }
    }

    pub fn reset_cursor(&mut self) {
        match &mut self.paginator().cursor_anchor {
            Seek::After { cache: posts } => {
                posts.push_front(Post::empty());
            }
            Seek::Back { cache: posts } => {
                posts.push_back(Post::empty());
            }
        };
    }

    pub fn paginator(&mut self) -> &mut Pagination {
        match self {
            Hot {
                paginator: params, ..
            } => params,
            New {
                paginator: params, ..
            } => params,
            Rising {
                paginator: params, ..
            } => params,
            Sort {
                paginator: params, ..
            } => params,
            Random { .. } => {
                panic!("pagination does not exist for `SubredditListing::Random`")
            }
        }
    }

    fn listing_tag(&self) -> &'static str {
        match self {
            Hot { .. } => "hot",
            New { .. } => "new",
            Random { .. } => "random",
            Rising { .. } => "rising",
            Sort { .. } => "sort",
        }
    }

    fn url_args(&self, pagination_arg: &Pagination) -> String {
        let mut buf = String::new();
        match &pagination_arg.cursor_anchor {
            Seek::Back { cache: posts } => {
                let mut args = String::new();
                if posts.len() == 0 {
                    args.push_str(format!("?after={}", "null").as_str());
                } else {
                    let post = posts.get(posts.len() - 1).unwrap();
                    let cursor_arg = if post.id().is_empty() {
                        "null".to_string()
                    } else {
                        format!("t3_{}", post.id().to_string())
                    };
                    args.push_str(format!("?after={}", cursor_arg).as_str());
                }
                buf.push_str(args.as_str())
            }
            Seek::After { cache: posts } => {
                let mut args = String::new();
                if posts.len() == 0 {
                    args.push_str(format!("?before={}", "null").as_str());
                } else {
                    let post = posts.get(posts.len() - 1).unwrap();
                    let cursor_arg = if post.id().is_empty() {
                        "null".to_string()
                    } else {
                        format!("t3_{}", post.id().to_string())
                    };
                    args.push_str(format!("?before={}", cursor_arg).as_str());
                }
                buf.push_str(args.as_str())
            }
        };
        buf.push_str(
            format!(
                "&count={}&limit={}&show={}",
                pagination_arg.seen_count, pagination_arg.limit, pagination_arg.show_rules
            )
            .as_str(),
        );
        buf
    }
}

impl ToString for Listing {
    fn to_string(&self) -> String {
        self.listing_tag().to_string()
    }
}
