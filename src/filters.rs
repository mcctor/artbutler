use crate::content::Post;

pub trait Filter {
    fn check(&self, post: &Post) -> bool;
}

pub struct VoteCountFilter;

impl Filter for VoteCountFilter {
    fn check(&self, post: &Post) -> bool {
        todo!()
    }
}

pub struct BlockedFilter;

impl Filter for BlockedFilter {
    fn check(&self, post: &Post) -> bool {
        todo!()
    }
}

pub struct SimilarFilter;

impl Filter for SimilarFilter {
    fn check(&self, post: &Post) -> bool {
        todo!()
    }
}
