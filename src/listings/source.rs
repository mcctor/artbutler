use async_trait::async_trait;
use std::collections::VecDeque;

use crate::content::Post;
use crate::listings::reddit::Listing;

#[async_trait]
pub trait ListingSource: Default + Send + Sync + Clone + 'static {
    async fn retrieve_posts(&mut self, listing: &mut Listing) -> reqwest::Result<VecDeque<Post>>;
}
