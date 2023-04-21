-- Your SQL goes here
CREATE TABLE artposts (
    id TEXT PRIMARY KEY,
    link TEXT NOT NULL,
    media_href TEXT NOT NULL,
    title TEXT NOT NULL,
    author TEXT NOT NULL,
    ups INT DEFAULT 0 NOT NULL,
    downs INT DEFAULT 0 NOT NULL
)