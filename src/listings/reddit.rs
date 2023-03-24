use std::collections::{HashMap, VecDeque};
use std::time::Duration;
use std::{env, vec};

use reqwest::{Client, Response, Result};
use serde_json::Value;
use tokio::time::Instant;

use crate::content::{Post, VoteCount};
use crate::curator::QUERY_RESULT_LIMIT;
use crate::listings::reddit::Listing::{Hot, New, Random, Rising, Sort};

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

#[derive(Debug)]
pub struct Reddit {
    cli: Client,
    has_token: Option<BearerToken>,
}

impl Reddit {
    pub fn new() -> Self {
        Reddit {
            cli: Client::new(),
            has_token: None,
        }
    }

    pub fn from(cli: &Client) -> Self {
        Reddit {
            cli: cli.clone(),
            has_token: None,
        }
    }

    pub async fn retrieve_posts(&mut self, listing: &mut Listing) -> Result<Vec<Post>> {
        let resp = self.request(listing).await?;
        let posts = self.serialize(resp, listing.result_limit()).await;
        if !posts.is_empty() {
            listing.update_paginator(&posts);
        }

        Ok(posts)
    }

    async fn authenticate(&mut self, auth_option: AuthTokenAction) -> Result<&BearerToken> {
        let client_id = env::var("CLIENT_ID").expect("CLIENT_ID not provided");
        let secret = env::var("SECRET").expect("SECRET not provided");
        let username = env::var("USER_NAME").expect("USERNAME not provided");
        let pass = env::var("PASSWORD").expect("PASSWORD not provided");

        let args = Reddit::auth_url_args(username, pass, self.has_token.as_ref(), auth_option);
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

    async fn authenticate_or_refresh(&mut self) -> Result<&BearerToken> {
        if let Some(t) = self.has_token.as_ref() {
            if Instant::now() > (t.expires - Duration::from_secs(60)) {
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

    async fn serialize(&self, resp: Response, result_count: u8) -> Vec<Post> {
        let raw_json = match resp.json::<Value>().await {
            Ok(v) => v,
            Err(_) => return vec![],
        };

        let mut posts = Vec::new();
        for i in 0..result_count {
            let post_raw = &raw_json["data"]["children"][i as usize]["data"];
            let post = self.parse_post(&post_raw);
            if *post.id() != "ul".to_string() {
                posts.push(post);
            }
        }

        posts
    }

    fn parse_post(&self, raw_json: &Value) -> Post {
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
    Before { cache: VecDeque<Post> },
}

#[derive(PartialEq, Debug, Clone, Eq, Hash)]
pub struct PaginationArg {
    pub cursor_anchor: Seek,
    pub limit: u8,
    pub seen_count: u8,
    pub show_rules: String,
}

impl PaginationArg {
    pub fn seek_front() -> Self {
        let mut cache = VecDeque::new();
        cache.push_back(Post::empty());
        PaginationArg {
            cursor_anchor: Seek::Before { cache },
            limit: 5,
            seen_count: 0,
            show_rules: "null".to_string(),
        }
    }
}

#[derive(PartialEq, Debug, Clone, Eq, Hash)]
pub enum Listing {
    Hot {
        subreddit: Subreddit,
        params: PaginationArg,
    },
    New {
        subreddit: Subreddit,
        params: PaginationArg,
    },
    Rising {
        subreddit: Subreddit,
        params: PaginationArg,
    },
    Sort {
        subreddit: Subreddit,
        params: PaginationArg,
        time: Time,
    },
    Random {
        subreddit: Subreddit,
    },
}

impl Listing {
    pub fn from(listing_name: &str, sub: Subreddit) -> Listing {
        match listing_name {
            "hot" => Hot {
                subreddit: sub,
                params: PaginationArg::seek_front(),
            },
            "new" => New {
                subreddit: sub,
                params: PaginationArg::seek_front(),
            },
            "rising" => Rising {
                subreddit: sub,
                params: PaginationArg::seek_front(),
            },
            "sort" => Sort {
                subreddit: sub,
                params: PaginationArg::seek_front(),
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
            Hot { subreddit, params } => {
                href_buf.push_str(format!("/r/{}/", subreddit.name()).as_str());
                href_buf.push_str(self.listing_tag());
                href_buf.push_str(self.url_args(params).as_str());
            }
            New { subreddit, params } => {
                href_buf.push_str(format!("/r/{}/", subreddit.name()).as_str());
                href_buf.push_str(self.listing_tag());
                href_buf.push_str(self.url_args(params).as_str());
            }
            Rising { subreddit, params } => {
                href_buf.push_str(format!("/r/{}/", subreddit.name()).as_str());
                href_buf.push_str(self.listing_tag());
                href_buf.push_str(self.url_args(params).as_str());
            }
            Sort {
                subreddit,
                time,
                params,
            } => {
                href_buf.push_str(format!("/r/{}/", subreddit.name()).as_str());
                href_buf.push_str(self.listing_tag());
                href_buf.push_str(self.url_args(params).as_str());
            }
            Random { subreddit } => {
                href_buf.push_str(format!("/r/{}/", subreddit.name()).as_str());
                href_buf.push_str(self.listing_tag());
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

    pub fn update_paginator(&mut self, with: &Vec<Post>) {
        match self {
            Random { .. } => (),
            listing => match &mut listing.paginator().cursor_anchor {
                Seek::After { cache: posts } => {
                    posts.clear();
                    for post in with {
                        posts.push_front(post.clone());
                    }
                }
                Seek::Before { cache: posts } => {
                    posts.clear();
                    for post in with {
                        posts.push_back(post.clone());
                    }
                }
            },
        };
    }

    pub fn result_limit(&mut self) -> u8 {
        match self {
            Random { .. } => 1,
            listing => listing.paginator().limit,
        }
    }

    pub fn set_anchor_post(&mut self, p: Post) {
        match &mut self.paginator().cursor_anchor {
            Seek::After { cache: posts } => {
                posts.push_front(p);
            }
            Seek::Before { cache: posts } => {
                posts.push_back(p);
            }
        };
    }

    pub fn paginator(&mut self) -> &mut PaginationArg {
        match self {
            Hot { params, .. } => params,
            New { params, .. } => params,
            Rising { params, .. } => params,
            Sort { params, .. } => params,
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

    fn url_args(&self, pagination_arg: &PaginationArg) -> String {
        let mut buf = String::new();
        match &pagination_arg.cursor_anchor {
            Seek::After { cache: posts } => {
                let mut args = String::new();
                if posts.len() == 0 {
                    args.push_str(format!("?after={}", "null").as_str());
                } else {
                    let post = posts.get(posts.len() - 1).unwrap();
                    args.push_str(format!("?after={}", format!("t3_{}", post.id())).as_str());
                }
                buf.push_str(args.as_str())
            }
            Seek::Before { cache: posts } => {
                let mut args = String::new();
                if posts.len() == 0 {
                    args.push_str(format!("?before={}", "null").as_str());
                } else {
                    let post = posts.get(0).unwrap();
                    args.push_str(format!("?before={}", format!("t3_{}", post.id())).as_str());
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

impl Default for Listing {
    fn default() -> Self {
        New {
            subreddit: Subreddit::from("art"),
            params: PaginationArg {
                cursor_anchor: Seek::After {
                    cache: VecDeque::new(),
                },
                limit: QUERY_RESULT_LIMIT,
                seen_count: 0,
                show_rules: "null".to_string(),
            },
        }
    }
}

impl ToString for Listing {
    fn to_string(&self) -> String {
        self.listing_tag().to_string()
    }
}
