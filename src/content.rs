use std::fmt;


#[derive(Debug)]
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

#[derive(Debug)]
pub struct Post {
    id: String,
    pub media_href: String,
    title: String,
    author: String,
    comments: Vec<String>,
    votes: VoteCount,
}

impl Post {
    pub fn new(
        id: String,
        media_href: String,
        author: String,
        title: String,
        votecount: VoteCount,
        comment_count: usize,
    ) -> Self {
        Post {
            id,
            media_href,
            title,
            author,
            comments: Vec::with_capacity(comment_count),
            votes: votecount,
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
