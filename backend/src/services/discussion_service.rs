use actix_session::Session;
use actix_web::HttpResponse;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use std::collections::HashMap;

use crate::entity::{
    courses, module_discussion_replies, module_discussion_threads, module_discussion_topics,
    modules, users,
};
use crate::models::discussion::{
    CreateDiscussionReply, CreateDiscussionThread, CreateDiscussionTopic, DiscussionAuthor,
    DiscussionReplyPayload, DiscussionThreadDetailPayload, DiscussionThreadPayload,
    DiscussionTopicPayload, PaginatedDiscussionPayload, UpdateDiscussionThread,
    UpdateDiscussionTopic,
};
use crate::services::auth_helpers::{get_user_id, is_enrolled};
use crate::services::course_service::{can_manage_course, get_session_user, has_role};

fn clean_required(value: &str, field: &str) -> Result<String, HttpResponse> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(HttpResponse::BadRequest().body(format!("{} is required", field)))
    } else {
        Ok(trimmed.to_string())
    }
}

fn display_name(user: &users::Model) -> String {
    format!("{} {}", user.first_name, user.last_name)
        .trim()
        .to_string()
}

async fn author_for_user(
    db: &DatabaseConnection,
    user_id: i32,
) -> Result<DiscussionAuthor, HttpResponse> {
    let user = users::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding user: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("User not found"))?;

    Ok(DiscussionAuthor {
        user_id: user.user_id,
        name: display_name(&user),
    })
}

async fn author_map(
    db: &DatabaseConnection,
    user_ids: Vec<i32>,
) -> Result<HashMap<i32, DiscussionAuthor>, HttpResponse> {
    if user_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let users = users::Entity::find()
        .filter(users::Column::UserId.is_in(user_ids))
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!(
                "Database error finding discussion authors: {}",
                err
            ))
        })?;

    Ok(users
        .into_iter()
        .map(|user| {
            (
                user.user_id,
                DiscussionAuthor {
                    user_id: user.user_id,
                    name: display_name(&user),
                },
            )
        })
        .collect())
}

pub async fn get_module_forum_course(
    db: &DatabaseConnection,
    module_id: i32,
) -> Result<(modules::Model, courses::Model), HttpResponse> {
    let module = modules::Entity::find_by_id(module_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding module: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Module not found"))?;

    let course = courses::Entity::find_by_id(module.course_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding course: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Course not found"))?;

    Ok((module, course))
}

async fn topic_module_course(
    db: &DatabaseConnection,
    topic_id: i32,
) -> Result<
    (
        module_discussion_topics::Model,
        modules::Model,
        courses::Model,
    ),
    HttpResponse,
> {
    let topic = module_discussion_topics::Entity::find_by_id(topic_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding discussion topic: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Discussion topic not found"))?;

    let (module, course) = get_module_forum_course(db, topic.module_id).await?;
    Ok((topic, module, course))
}

async fn thread_topic_module_course(
    db: &DatabaseConnection,
    thread_id: i32,
) -> Result<
    (
        module_discussion_threads::Model,
        module_discussion_topics::Model,
        modules::Model,
        courses::Model,
    ),
    HttpResponse,
> {
    let thread = module_discussion_threads::Entity::find_by_id(thread_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding discussion thread: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Discussion thread not found"))?;

    let (topic, module, course) = topic_module_course(db, thread.topic_id).await?;
    Ok((thread, topic, module, course))
}

async fn can_access_course_for_forum(
    db: &DatabaseConnection,
    session: &Session,
    course: &courses::Model,
) -> Result<bool, HttpResponse> {
    if can_manage_course(db, session, course).await? {
        return Ok(true);
    }

    let user_id = get_user_id(session)?;
    is_enrolled(db, user_id, course.course_id).await
}

pub async fn can_create_forum_post(
    db: &DatabaseConnection,
    session: &Session,
    course: &courses::Model,
) -> Result<bool, HttpResponse> {
    can_access_course_for_forum(db, session, course).await
}

async fn can_delete_discussion_topic(
    db: &DatabaseConnection,
    session: &Session,
    course: &courses::Model,
) -> Result<bool, HttpResponse> {
    if has_role(session, "LMS Admin") {
        return Ok(true);
    }

    if !has_role(session, "Organisation Admin") {
        return Ok(false);
    }

    let user = get_session_user(db, session).await?;
    Ok(user.org_id.is_some() && user.org_id == course.org_id)
}

fn can_edit_owned(user_id: i32, author_id: i32, status: &str) -> bool {
    user_id == author_id && status == "open"
}

fn is_lms_admin(session: &Session) -> bool {
    has_role(session, "LMS Admin")
}

fn normalize_page(page: u64, page_size: u64) -> (u64, u64, u64) {
    let page = page.max(1);
    let page_size = page_size.clamp(1, 50);
    let offset = (page - 1) * page_size;
    (page, page_size, offset)
}

fn total_pages(total: u64, page_size: u64) -> u64 {
    if total == 0 {
        1
    } else {
        total.div_ceil(page_size)
    }
}

pub async fn list_topics(
    db: &DatabaseConnection,
    session: &Session,
    module_id: i32,
) -> Result<Vec<DiscussionTopicPayload>, HttpResponse> {
    let (_, course) = get_module_forum_course(db, module_id).await?;
    if !can_access_course_for_forum(db, session, &course).await? {
        return Err(HttpResponse::Forbidden().body("You cannot view this module discussion"));
    }

    let can_manage = can_manage_course(db, session, &course).await?;
    let can_delete = can_delete_discussion_topic(db, session, &course).await?;
    let can_create_thread = can_create_forum_post(db, session, &course).await?;
    let topics = module_discussion_topics::Entity::find()
        .filter(module_discussion_topics::Column::ModuleId.eq(module_id))
        .order_by_asc(module_discussion_topics::Column::CreatedAt)
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding discussion topics: {}", err))
        })?;

    let mut payloads = Vec::with_capacity(topics.len());

    for topic in topics {
        let threads = module_discussion_threads::Entity::find()
            .filter(module_discussion_threads::Column::TopicId.eq(topic.topic_id))
            .filter(module_discussion_threads::Column::HiddenAt.is_null())
            .all(db)
            .await
            .map_err(|err| {
                HttpResponse::InternalServerError().body(format!(
                    "Database error finding discussion threads: {}",
                    err
                ))
            })?;

        let mut reply_total = 0;
        let mut last_post_author_id = None;
        let mut last_post_at = None;

        for thread in &threads {
            let reply_count = module_discussion_replies::Entity::find()
                .filter(module_discussion_replies::Column::ThreadId.eq(thread.thread_id))
                .filter(module_discussion_replies::Column::DeletedAt.is_null())
                .count(db)
                .await
                .map_err(|err| {
                    HttpResponse::InternalServerError().body(format!(
                        "Database error counting discussion replies: {}",
                        err
                    ))
                })?;
            reply_total += reply_count;

            if last_post_at.is_none_or(|current| thread.updated_at > current) {
                last_post_at = Some(thread.updated_at);
                last_post_author_id = Some(thread.author_id);
            }
        }

        let author = author_for_user(db, topic.created_by).await?;
        let last_post_author = match last_post_author_id {
            Some(author_id) => Some(author_for_user(db, author_id).await?),
            None => None,
        };

        payloads.push(DiscussionTopicPayload {
            topic_id: topic.topic_id,
            module_id: topic.module_id,
            title: topic.title,
            description: topic.description,
            is_locked: topic.is_locked,
            created_at: topic.created_at,
            updated_at: topic.updated_at,
            author,
            thread_count: threads.len() as u64,
            post_count: threads.len() as u64 + reply_total,
            last_post_author,
            last_post_at,
            can_create_thread: can_create_thread && !topic.is_locked,
            can_manage,
            can_delete,
        });
    }

    Ok(payloads)
}

pub async fn list_course_topics(
    db: &DatabaseConnection,
    session: &Session,
    course_id: i32,
) -> Result<Vec<DiscussionTopicPayload>, HttpResponse> {
    let course = courses::Entity::find_by_id(course_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding course: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Course not found"))?;

    if !can_access_course_for_forum(db, session, &course).await? {
        return Err(HttpResponse::Forbidden().body("You cannot view this course discussion"));
    }

    let module_rows = modules::Entity::find()
        .filter(modules::Column::CourseId.eq(course_id))
        .order_by_asc(modules::Column::Position)
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding course modules: {}", err))
        })?;

    let mut payloads = Vec::new();
    for module in module_rows {
        payloads.extend(list_topics(db, session, module.module_id).await?);
    }

    payloads.sort_by(|first, second| second.updated_at.cmp(&first.updated_at));
    Ok(payloads)
}

pub async fn list_course_topics_paginated(
    db: &DatabaseConnection,
    session: &Session,
    course_id: i32,
    page: u64,
    page_size: u64,
) -> Result<PaginatedDiscussionPayload<DiscussionTopicPayload>, HttpResponse> {
    let (page, page_size, offset) = normalize_page(page, page_size);
    let topics = list_course_topics(db, session, course_id).await?;
    let total = topics.len() as u64;
    let items = topics
        .into_iter()
        .skip(offset as usize)
        .take(page_size as usize)
        .collect();

    Ok(PaginatedDiscussionPayload {
        items,
        page,
        page_size,
        total,
        total_pages: total_pages(total, page_size),
    })
}

pub async fn create_topic(
    db: &DatabaseConnection,
    session: &Session,
    data: CreateDiscussionTopic,
) -> Result<DiscussionTopicPayload, HttpResponse> {
    let (_, course) = get_module_forum_course(db, data.module_id).await?;
    if !can_manage_course(db, session, &course).await? {
        return Err(
            HttpResponse::Forbidden().body("Only course staff can create discussion topics")
        );
    }

    let user_id = get_user_id(session)?;
    let topic = module_discussion_topics::ActiveModel {
        module_id: Set(data.module_id),
        created_by: Set(user_id),
        title: Set(clean_required(&data.title, "Title")?),
        description: Set(data
            .description
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(|err| {
        HttpResponse::InternalServerError()
            .body(format!("Database error creating discussion topic: {}", err))
    })?;

    let mut topics = list_topics(db, session, topic.module_id).await?;
    topics
        .drain(..)
        .find(|payload| payload.topic_id == topic.topic_id)
        .ok_or_else(|| HttpResponse::InternalServerError().body("Created topic not found"))
}

pub async fn update_topic(
    db: &DatabaseConnection,
    session: &Session,
    topic_id: i32,
    data: UpdateDiscussionTopic,
) -> Result<DiscussionTopicPayload, HttpResponse> {
    let (topic, _, course) = topic_module_course(db, topic_id).await?;
    if !can_manage_course(db, session, &course).await? {
        return Err(
            HttpResponse::Forbidden().body("Only course staff can update discussion topics")
        );
    }

    let mut active: module_discussion_topics::ActiveModel = topic.clone().into();
    if let Some(title) = data.title {
        active.title = Set(clean_required(&title, "Title")?);
    }
    if let Some(description) = data.description {
        active.description =
            Set(Some(description.trim().to_string()).filter(|value| !value.is_empty()));
    }
    if let Some(is_locked) = data.is_locked {
        active.is_locked = Set(is_locked);
    }
    active.updated_at = Set(Utc::now().fixed_offset());

    let updated = active.update(db).await.map_err(|err| {
        HttpResponse::InternalServerError()
            .body(format!("Database error updating discussion topic: {}", err))
    })?;

    let mut topics = list_topics(db, session, updated.module_id).await?;
    topics
        .drain(..)
        .find(|payload| payload.topic_id == updated.topic_id)
        .ok_or_else(|| HttpResponse::InternalServerError().body("Updated topic not found"))
}

pub async fn delete_topic(
    db: &DatabaseConnection,
    session: &Session,
    topic_id: i32,
) -> Result<(), HttpResponse> {
    let (topic, _, course) = topic_module_course(db, topic_id).await?;
    if !can_delete_discussion_topic(db, session, &course).await? {
        return Err(HttpResponse::Forbidden()
            .body("Only organisation admins and LMS admins can delete discussion topics"));
    }

    module_discussion_topics::Entity::delete_by_id(topic.topic_id)
        .exec(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error deleting discussion topic: {}", err))
        })?;

    Ok(())
}

pub async fn list_threads(
    db: &DatabaseConnection,
    session: &Session,
    topic_id: i32,
) -> Result<Vec<DiscussionThreadPayload>, HttpResponse> {
    let (topic, _, course) = topic_module_course(db, topic_id).await?;
    if !can_access_course_for_forum(db, session, &course).await? {
        return Err(HttpResponse::Forbidden().body("You cannot view this discussion topic"));
    }

    let user_id = get_user_id(session)?;
    let can_manage = can_manage_course(db, session, &course).await?;
    let lms_admin = is_lms_admin(session);
    let can_post = can_create_forum_post(db, session, &course).await? && !topic.is_locked;
    let threads = module_discussion_threads::Entity::find()
        .filter(module_discussion_threads::Column::TopicId.eq(topic_id))
        .filter(module_discussion_threads::Column::HiddenAt.is_null())
        .order_by_desc(module_discussion_threads::Column::UpdatedAt)
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!(
                "Database error finding discussion threads: {}",
                err
            ))
        })?;

    let authors = author_map(db, threads.iter().map(|thread| thread.author_id).collect()).await?;
    let mut payloads = Vec::with_capacity(threads.len());

    for thread in threads {
        let reply_count = module_discussion_replies::Entity::find()
            .filter(module_discussion_replies::Column::ThreadId.eq(thread.thread_id))
            .filter(module_discussion_replies::Column::DeletedAt.is_null())
            .count(db)
            .await
            .map_err(|err| {
                HttpResponse::InternalServerError().body(format!(
                    "Database error counting discussion replies: {}",
                    err
                ))
            })?;

        payloads.push(DiscussionThreadPayload {
            thread_id: thread.thread_id,
            topic_id: thread.topic_id,
            title: thread.title,
            body: thread.body,
            status: thread.status.clone(),
            view_count: thread.view_count,
            created_at: thread.created_at,
            updated_at: thread.updated_at,
            author: authors
                .get(&thread.author_id)
                .cloned()
                .unwrap_or(DiscussionAuthor {
                    user_id: thread.author_id,
                    name: "Unknown user".to_string(),
                }),
            reply_count,
            can_edit: lms_admin || can_edit_owned(user_id, thread.author_id, &thread.status),
            can_close: thread.status == "open" && (can_manage || user_id == thread.author_id),
            can_hide: lms_admin || (thread.status == "closed" && can_manage),
            can_reply: thread.status == "open" && can_post,
        });
    }

    Ok(payloads)
}

pub async fn list_threads_paginated(
    db: &DatabaseConnection,
    session: &Session,
    topic_id: i32,
    page: u64,
    page_size: u64,
) -> Result<PaginatedDiscussionPayload<DiscussionThreadPayload>, HttpResponse> {
    let (topic, _, course) = topic_module_course(db, topic_id).await?;
    if !can_access_course_for_forum(db, session, &course).await? {
        return Err(HttpResponse::Forbidden().body("You cannot view this discussion topic"));
    }

    let (page, page_size, offset) = normalize_page(page, page_size);
    let user_id = get_user_id(session)?;
    let can_manage = can_manage_course(db, session, &course).await?;
    let lms_admin = is_lms_admin(session);
    let can_post = can_create_forum_post(db, session, &course).await? && !topic.is_locked;
    let total = module_discussion_threads::Entity::find()
        .filter(module_discussion_threads::Column::TopicId.eq(topic_id))
        .filter(module_discussion_threads::Column::HiddenAt.is_null())
        .count(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!(
                "Database error counting discussion threads: {}",
                err
            ))
        })?;
    let threads = module_discussion_threads::Entity::find()
        .filter(module_discussion_threads::Column::TopicId.eq(topic_id))
        .filter(module_discussion_threads::Column::HiddenAt.is_null())
        .order_by_desc(module_discussion_threads::Column::UpdatedAt)
        .offset(offset)
        .limit(page_size)
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!(
                "Database error finding discussion threads: {}",
                err
            ))
        })?;

    let authors = author_map(db, threads.iter().map(|thread| thread.author_id).collect()).await?;
    let mut items = Vec::with_capacity(threads.len());

    for thread in threads {
        let reply_count = module_discussion_replies::Entity::find()
            .filter(module_discussion_replies::Column::ThreadId.eq(thread.thread_id))
            .filter(module_discussion_replies::Column::DeletedAt.is_null())
            .count(db)
            .await
            .map_err(|err| {
                HttpResponse::InternalServerError().body(format!(
                    "Database error counting discussion replies: {}",
                    err
                ))
            })?;

        items.push(DiscussionThreadPayload {
            thread_id: thread.thread_id,
            topic_id: thread.topic_id,
            title: thread.title,
            body: thread.body,
            status: thread.status.clone(),
            view_count: thread.view_count,
            created_at: thread.created_at,
            updated_at: thread.updated_at,
            author: authors
                .get(&thread.author_id)
                .cloned()
                .unwrap_or(DiscussionAuthor {
                    user_id: thread.author_id,
                    name: "Unknown user".to_string(),
                }),
            reply_count,
            can_edit: lms_admin || can_edit_owned(user_id, thread.author_id, &thread.status),
            can_close: thread.status == "open" && (can_manage || user_id == thread.author_id),
            can_hide: lms_admin || (thread.status == "closed" && can_manage),
            can_reply: thread.status == "open" && can_post,
        });
    }

    Ok(PaginatedDiscussionPayload {
        items,
        page,
        page_size,
        total,
        total_pages: total_pages(total, page_size),
    })
}

pub async fn create_thread(
    db: &DatabaseConnection,
    session: &Session,
    topic_id: i32,
    data: CreateDiscussionThread,
) -> Result<DiscussionThreadPayload, HttpResponse> {
    let (topic, _, course) = topic_module_course(db, topic_id).await?;
    if topic.is_locked {
        return Err(HttpResponse::Forbidden().body("This discussion topic is locked"));
    }
    if !can_create_forum_post(db, session, &course).await? {
        return Err(HttpResponse::Forbidden().body("You cannot create a thread in this topic"));
    }

    let user_id = get_user_id(session)?;
    let thread = module_discussion_threads::ActiveModel {
        topic_id: Set(topic_id),
        author_id: Set(user_id),
        title: Set(clean_required(&data.title, "Title")?),
        body: Set(clean_required(&data.body, "Body")?),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(|err| {
        HttpResponse::InternalServerError().body(format!(
            "Database error creating discussion thread: {}",
            err
        ))
    })?;

    let mut threads = list_threads(db, session, topic_id).await?;
    threads
        .drain(..)
        .find(|payload| payload.thread_id == thread.thread_id)
        .ok_or_else(|| HttpResponse::InternalServerError().body("Created thread not found"))
}

pub async fn get_thread_detail(
    db: &DatabaseConnection,
    session: &Session,
    thread_id: i32,
    page: u64,
    page_size: u64,
) -> Result<DiscussionThreadDetailPayload, HttpResponse> {
    let (thread, _, _, course) = thread_topic_module_course(db, thread_id).await?;
    if thread.hidden_at.is_some() {
        return Err(HttpResponse::NotFound().body("Discussion thread not found"));
    }
    if !can_access_course_for_forum(db, session, &course).await? {
        return Err(HttpResponse::Forbidden().body("You cannot view this discussion thread"));
    }

    let mut active: module_discussion_threads::ActiveModel = thread.clone().into();
    active.view_count = Set(thread.view_count + 1);
    let _ = active.update(db).await;

    let mut thread_payload = list_threads(db, session, thread.topic_id)
        .await?
        .into_iter()
        .find(|payload| payload.thread_id == thread_id)
        .ok_or_else(|| HttpResponse::NotFound().body("Discussion thread not found"))?;
    thread_payload.view_count += 1;

    let (page, page_size, offset) = normalize_page(page, page_size);
    let can_manage = can_manage_course(db, session, &course).await?;
    let replies_total = module_discussion_replies::Entity::find()
        .filter(module_discussion_replies::Column::ThreadId.eq(thread_id))
        .filter(module_discussion_replies::Column::ParentReplyId.is_null())
        .filter(module_discussion_replies::Column::DeletedAt.is_null())
        .count(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!(
                "Database error counting discussion replies: {}",
                err
            ))
        })?;
    let top_level_replies = module_discussion_replies::Entity::find()
        .filter(module_discussion_replies::Column::ThreadId.eq(thread_id))
        .filter(module_discussion_replies::Column::ParentReplyId.is_null())
        .filter(module_discussion_replies::Column::DeletedAt.is_null())
        .order_by_asc(module_discussion_replies::Column::CreatedAt)
        .offset(offset)
        .limit(page_size)
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!(
                "Database error finding discussion replies: {}",
                err
            ))
        })?;
    let mut replies = top_level_replies.clone();
    let mut frontier: Vec<i32> = top_level_replies
        .iter()
        .map(|reply| reply.reply_id)
        .collect();

    while !frontier.is_empty() {
        let children = module_discussion_replies::Entity::find()
            .filter(module_discussion_replies::Column::ThreadId.eq(thread_id))
            .filter(module_discussion_replies::Column::ParentReplyId.is_in(frontier.clone()))
            .filter(module_discussion_replies::Column::DeletedAt.is_null())
            .order_by_asc(module_discussion_replies::Column::CreatedAt)
            .all(db)
            .await
            .map_err(|err| {
                HttpResponse::InternalServerError().body(format!(
                    "Database error finding child discussion replies: {}",
                    err
                ))
            })?;

        frontier = children.iter().map(|reply| reply.reply_id).collect();
        replies.extend(children);
    }
    replies.sort_by(|first, second| first.created_at.cmp(&second.created_at));

    let authors = author_map(db, replies.iter().map(|reply| reply.author_id).collect()).await?;
    let reply_payloads = replies
        .into_iter()
        .map(|reply| DiscussionReplyPayload {
            reply_id: reply.reply_id,
            thread_id: reply.thread_id,
            parent_reply_id: reply.parent_reply_id,
            body: reply.body,
            deleted_at: reply.deleted_at,
            created_at: reply.created_at,
            updated_at: reply.updated_at,
            author: authors
                .get(&reply.author_id)
                .cloned()
                .unwrap_or(DiscussionAuthor {
                    user_id: reply.author_id,
                    name: "Unknown user".to_string(),
                }),
            can_delete: can_manage && reply.deleted_at.is_none(),
        })
        .collect();

    Ok(DiscussionThreadDetailPayload {
        thread: thread_payload,
        replies: reply_payloads,
        replies_page: page,
        replies_page_size: page_size,
        replies_total,
        replies_total_pages: total_pages(replies_total, page_size),
    })
}

pub async fn update_thread(
    db: &DatabaseConnection,
    session: &Session,
    thread_id: i32,
    data: UpdateDiscussionThread,
) -> Result<DiscussionThreadPayload, HttpResponse> {
    let (thread, _, _, _course) = thread_topic_module_course(db, thread_id).await?;
    let user_id = get_user_id(session)?;
    if !is_lms_admin(session) && !can_edit_owned(user_id, thread.author_id, &thread.status) {
        return Err(HttpResponse::Forbidden().body("You cannot edit this thread"));
    }

    let mut active: module_discussion_threads::ActiveModel = thread.clone().into();
    if let Some(title) = data.title {
        active.title = Set(clean_required(&title, "Title")?);
    }
    if let Some(body) = data.body {
        active.body = Set(clean_required(&body, "Body")?);
    }
    active.updated_at = Set(Utc::now().fixed_offset());

    let updated = active.update(db).await.map_err(|err| {
        HttpResponse::InternalServerError().body(format!(
            "Database error updating discussion thread: {}",
            err
        ))
    })?;

    list_threads(db, session, updated.topic_id)
        .await?
        .into_iter()
        .find(|payload| payload.thread_id == updated.thread_id)
        .ok_or_else(|| HttpResponse::InternalServerError().body("Updated thread not found"))
}

pub async fn close_thread(
    db: &DatabaseConnection,
    session: &Session,
    thread_id: i32,
) -> Result<DiscussionThreadPayload, HttpResponse> {
    let (thread, _, _, course) = thread_topic_module_course(db, thread_id).await?;
    let user_id = get_user_id(session)?;
    let can_manage = can_manage_course(db, session, &course).await?;

    if thread.status != "open" {
        return Err(HttpResponse::BadRequest().body("Thread is already closed"));
    }
    if !can_manage && user_id != thread.author_id {
        return Err(HttpResponse::Forbidden().body("You cannot close this thread"));
    }

    let now = Utc::now().fixed_offset();
    let mut active: module_discussion_threads::ActiveModel = thread.clone().into();
    active.status = Set("closed".to_string());
    active.closed_by = Set(Some(user_id));
    active.closed_at = Set(Some(now));
    active.updated_at = Set(now);

    let updated = active.update(db).await.map_err(|err| {
        HttpResponse::InternalServerError()
            .body(format!("Database error closing discussion thread: {}", err))
    })?;

    list_threads(db, session, updated.topic_id)
        .await?
        .into_iter()
        .find(|payload| payload.thread_id == updated.thread_id)
        .ok_or_else(|| HttpResponse::InternalServerError().body("Closed thread not found"))
}

pub async fn hide_thread(
    db: &DatabaseConnection,
    session: &Session,
    thread_id: i32,
) -> Result<(), HttpResponse> {
    let (thread, _, _, course) = thread_topic_module_course(db, thread_id).await?;
    let user_id = get_user_id(session)?;
    let lms_admin = is_lms_admin(session);

    if !lms_admin && !can_manage_course(db, session, &course).await? {
        return Err(HttpResponse::Forbidden()
            .body("Only course staff can remove closed discussion threads"));
    }
    if !lms_admin && thread.status != "closed" {
        return Err(
            HttpResponse::BadRequest().body("Only closed discussion threads can be removed")
        );
    }
    if thread.hidden_at.is_some() {
        return Ok(());
    }

    let now = Utc::now().fixed_offset();
    let mut active: module_discussion_threads::ActiveModel = thread.into();
    active.hidden_by = Set(Some(user_id));
    active.hidden_at = Set(Some(now));
    active.updated_at = Set(now);

    active.update(db).await.map_err(|err| {
        HttpResponse::InternalServerError().body(format!(
            "Database error removing discussion thread: {}",
            err
        ))
    })?;

    Ok(())
}

pub async fn create_reply(
    db: &DatabaseConnection,
    session: &Session,
    thread_id: i32,
    data: CreateDiscussionReply,
) -> Result<DiscussionReplyPayload, HttpResponse> {
    let (thread, topic, _, course) = thread_topic_module_course(db, thread_id).await?;
    if topic.is_locked || thread.status != "open" {
        return Err(HttpResponse::Forbidden().body("This discussion is closed"));
    }
    if !can_create_forum_post(db, session, &course).await? {
        return Err(HttpResponse::Forbidden().body("You cannot reply to this thread"));
    }

    if let Some(parent_reply_id) = data.parent_reply_id {
        let parent_reply = module_discussion_replies::Entity::find_by_id(parent_reply_id)
            .one(db)
            .await
            .map_err(|err| {
                HttpResponse::InternalServerError()
                    .body(format!("Database error finding parent reply: {}", err))
            })?
            .ok_or_else(|| HttpResponse::BadRequest().body("Parent reply not found"))?;

        if parent_reply.thread_id != thread_id || parent_reply.deleted_at.is_some() {
            return Err(
                HttpResponse::BadRequest().body("Parent reply does not belong to this thread")
            );
        }
    }

    let user_id = get_user_id(session)?;
    let reply = module_discussion_replies::ActiveModel {
        thread_id: Set(thread_id),
        parent_reply_id: Set(data.parent_reply_id),
        author_id: Set(user_id),
        body: Set(clean_required(&data.body, "Reply")?),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(|err| {
        HttpResponse::InternalServerError()
            .body(format!("Database error creating discussion reply: {}", err))
    })?;

    let mut active_thread: module_discussion_threads::ActiveModel = thread.into();
    active_thread.updated_at = Set(Utc::now().fixed_offset());
    let _ = active_thread.update(db).await;

    Ok(DiscussionReplyPayload {
        reply_id: reply.reply_id,
        thread_id: reply.thread_id,
        parent_reply_id: reply.parent_reply_id,
        body: reply.body,
        deleted_at: reply.deleted_at,
        created_at: reply.created_at,
        updated_at: reply.updated_at,
        author: author_for_user(db, reply.author_id).await?,
        can_delete: can_manage_course(db, session, &course).await?,
    })
}

pub async fn delete_reply(
    db: &DatabaseConnection,
    session: &Session,
    reply_id: i32,
) -> Result<(), HttpResponse> {
    let reply = module_discussion_replies::Entity::find_by_id(reply_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding discussion reply: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("Discussion reply not found"))?;

    let (_, _, _, course) = thread_topic_module_course(db, reply.thread_id).await?;
    if !can_manage_course(db, session, &course).await? {
        return Err(
            HttpResponse::Forbidden().body("Only course staff can delete discussion replies")
        );
    }
    if reply.deleted_at.is_some() {
        return Ok(());
    }

    let now = Utc::now().fixed_offset();
    let mut reply_ids = vec![reply.reply_id];
    let mut frontier = vec![reply.reply_id];

    while !frontier.is_empty() {
        let children = module_discussion_replies::Entity::find()
            .filter(module_discussion_replies::Column::ParentReplyId.is_in(frontier.clone()))
            .filter(module_discussion_replies::Column::DeletedAt.is_null())
            .all(db)
            .await
            .map_err(|err| {
                HttpResponse::InternalServerError().body(format!(
                    "Database error finding child discussion replies: {}",
                    err
                ))
            })?;

        frontier = children.iter().map(|child| child.reply_id).collect();
        reply_ids.extend(frontier.iter().copied());
    }

    let replies_to_delete = module_discussion_replies::Entity::find()
        .filter(module_discussion_replies::Column::ReplyId.is_in(reply_ids))
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError().body(format!(
                "Database error finding discussion replies to delete: {}",
                err
            ))
        })?;

    for reply_to_delete in replies_to_delete {
        let mut active: module_discussion_replies::ActiveModel = reply_to_delete.into();
        active.deleted_at = Set(Some(now));
        active.updated_at = Set(now);

        active.update(db).await.map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error deleting discussion reply: {}", err))
        })?;
    }

    Ok(())
}
