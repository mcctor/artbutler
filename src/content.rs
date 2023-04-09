use std::fmt::Formatter;
use std::hash::Hasher;

#[derive(PartialEq, Debug, Clone, Eq, Hash)]
pub struct VoteCount(i32, i32);

impl VoteCount {
    pub fn from(upvote: i32, downvote: i32) -> Self {
        VoteCount(upvote, downvote)
    }

    pub fn ups(&self) -> i32 {
        self.0
    }

    pub fn downs(&self) -> i32 {
        self.1
    }
}

#[derive(Debug, Clone, Eq)]
pub struct Post {
    id: String,
    pub link: String,
    pub media_href: String,
    title: String,
    author: String,
    votes: VoteCount,
}

impl Post {
    pub fn new(
        id: String,
        link: String,
        media_href: String,
        author: String,
        title: String,
        votecount: VoteCount,
    ) -> Self {
        Post {
            id,
            link,
            media_href,
            title,
            author,
            votes: votecount,
        }
    }

    pub fn empty() -> Self {
        Post {
            id: String::new(),
            link: String::new(),
            media_href: String::new(),
            title: String::new(),
            author: String::new(),
            votes: VoteCount(0, 0),
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
        H: std::hash::Hasher,
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
