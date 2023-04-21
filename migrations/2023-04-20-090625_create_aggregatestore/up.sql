-- Your SQL goes here
-- Your SQL goes here
CREATE TABLE subscribed_listings (
    id SERIAL PRIMARY KEY,
    user_id BIGINT NOT NULL REFERENCES botclients (id),
    subreddit TEXT NOT NULL,
    category TEXT NOT NULL,
    head_post_id TEXT REFERENCES artposts(id)
)