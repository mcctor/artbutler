// @generated automatically by Diesel CLI.

diesel::table! {
    artposts (id) {
        id -> Text,
        link -> Text,
        media_href -> Text,
        title -> Text,
        author -> Text,
        ups -> Int4,
        downs -> Int4,
    }
}

diesel::table! {
    botclients (id) {
        id -> Int8,
        username -> Nullable<Text>,
        is_user -> Bool,
    }
}

diesel::table! {
    posts (id) {
        id -> Int4,
        title -> Varchar,
        body -> Text,
        published -> Bool,
    }
}

diesel::table! {
    subscribed_listings (id) {
        id -> Int4,
        user_id -> Int8,
        subreddit -> Text,
        category -> Text,
        head_post_id -> Nullable<Text>,
    }
}

diesel::joinable!(subscribed_listings -> artposts (head_post_id));
diesel::joinable!(subscribed_listings -> botclients (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    artposts,
    botclients,
    posts,
    subscribed_listings,
);
