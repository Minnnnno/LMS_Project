use sea_orm::prelude::DateTimeWithTimeZone;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct DiscussionPageQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[derive(Serialize)]
pub struct PaginatedDiscussionPayload<T> {
    pub items: Vec<T>,
    pub page: u64,
    pub page_size: u64,
    pub total: u64,
    pub total_pages: u64,
}

#[derive(Deserialize)]
pub struct CreateDiscussionTopic {
    pub module_id: i32,
    pub title: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateDiscussionTopic {
    pub title: Option<String>,
    pub description: Option<String>,
    pub is_locked: Option<bool>,
}

#[derive(Deserialize)]
pub struct CreateDiscussionThread {
    pub title: String,
    pub body: String,
}

#[derive(Deserialize)]
pub struct UpdateDiscussionThread {
    pub title: Option<String>,
    pub body: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateDiscussionReply {
    pub body: String,
    pub parent_reply_id: Option<i32>,
}

#[derive(Clone, Serialize)]
pub struct DiscussionAuthor {
    pub user_id: i32,
    pub name: String,
}

#[derive(Serialize)]
pub struct DiscussionTopicPayload {
    pub topic_id: i32,
    pub module_id: i32,
    pub title: String,
    pub description: Option<String>,
    pub is_locked: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub author: DiscussionAuthor,
    pub thread_count: u64,
    pub post_count: u64,
    pub last_post_author: Option<DiscussionAuthor>,
    pub last_post_at: Option<DateTimeWithTimeZone>,
    pub can_create_thread: bool,
    pub can_manage: bool,
    pub can_delete: bool,
}

#[derive(Serialize)]
pub struct DiscussionThreadPayload {
    pub thread_id: i32,
    pub topic_id: i32,
    pub title: String,
    pub body: String,
    pub status: String,
    pub view_count: i32,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub author: DiscussionAuthor,
    pub reply_count: u64,
    pub can_edit: bool,
    pub can_close: bool,
    pub can_hide: bool,
    pub can_reply: bool,
}

#[derive(Serialize)]
pub struct DiscussionReplyPayload {
    pub reply_id: i32,
    pub thread_id: i32,
    pub parent_reply_id: Option<i32>,
    pub body: String,
    pub deleted_at: Option<DateTimeWithTimeZone>,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    pub author: DiscussionAuthor,
    pub can_delete: bool,
}

#[derive(Serialize)]
pub struct DiscussionThreadDetailPayload {
    pub thread: DiscussionThreadPayload,
    pub replies: Vec<DiscussionReplyPayload>,
    pub replies_page: u64,
    pub replies_page_size: u64,
    pub replies_total: u64,
    pub replies_total_pages: u64,
}
