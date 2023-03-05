use std::fmt;


#[derive(PartialEq, Debug, Clone)]
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

#[derive(PartialEq, Debug, Clone)]
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
}

impl fmt::Display for Post {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        // implement formatter.
        Ok(())
    }
}
