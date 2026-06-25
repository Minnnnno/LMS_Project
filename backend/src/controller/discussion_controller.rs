use actix_session::Session;
use actix_web::{HttpResponse, Responder, delete, get, post, put, web};
use sea_orm::DatabaseConnection;

use crate::models::discussion::{
    CreateDiscussionReply, CreateDiscussionThread, CreateDiscussionTopic, DiscussionPageQuery,
    UpdateDiscussionThread, UpdateDiscussionTopic,
};
use crate::services::discussion_service;

#[get("/discussions/modules/{module_id}/topics")]
pub async fn list_discussion_topics(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    match discussion_service::list_topics(db.get_ref(), &session, path.into_inner()).await {
        Ok(topics) => HttpResponse::Ok().json(topics),
        Err(response) => response,
    }
}

#[get("/discussions/courses/{course_id}/topics")]
pub async fn list_course_discussion_topics(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    query: web::Query<DiscussionPageQuery>,
) -> impl Responder {
    match discussion_service::list_course_topics_paginated(
        db.get_ref(),
        &session,
        path.into_inner(),
        query.page.unwrap_or(1),
        query.page_size.unwrap_or(5),
    )
    .await
    {
        Ok(topics) => HttpResponse::Ok().json(topics),
        Err(response) => response,
    }
}

#[post("/discussions/topics")]
pub async fn create_discussion_topic(
    db: web::Data<DatabaseConnection>,
    session: Session,
    body: web::Json<CreateDiscussionTopic>,
) -> impl Responder {
    match discussion_service::create_topic(db.get_ref(), &session, body.into_inner()).await {
        Ok(topic) => HttpResponse::Ok().json(topic),
        Err(response) => response,
    }
}

#[put("/discussions/topics/{topic_id}")]
pub async fn update_discussion_topic(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    body: web::Json<UpdateDiscussionTopic>,
) -> impl Responder {
    match discussion_service::update_topic(
        db.get_ref(),
        &session,
        path.into_inner(),
        body.into_inner(),
    )
    .await
    {
        Ok(topic) => HttpResponse::Ok().json(topic),
        Err(response) => response,
    }
}

#[delete("/discussions/topics/{topic_id}")]
pub async fn delete_discussion_topic(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    match discussion_service::delete_topic(db.get_ref(), &session, path.into_inner()).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(response) => response,
    }
}

#[get("/discussions/topics/{topic_id}/threads")]
pub async fn list_discussion_threads(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    query: web::Query<DiscussionPageQuery>,
) -> impl Responder {
    match discussion_service::list_threads_paginated(
        db.get_ref(),
        &session,
        path.into_inner(),
        query.page.unwrap_or(1),
        query.page_size.unwrap_or(7),
    )
    .await
    {
        Ok(threads) => HttpResponse::Ok().json(threads),
        Err(response) => response,
    }
}

#[post("/discussions/topics/{topic_id}/threads")]
pub async fn create_discussion_thread(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    body: web::Json<CreateDiscussionThread>,
) -> impl Responder {
    match discussion_service::create_thread(
        db.get_ref(),
        &session,
        path.into_inner(),
        body.into_inner(),
    )
    .await
    {
        Ok(thread) => HttpResponse::Ok().json(thread),
        Err(response) => response,
    }
}

#[get("/discussions/threads/{thread_id}")]
pub async fn get_discussion_thread(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    query: web::Query<DiscussionPageQuery>,
) -> impl Responder {
    match discussion_service::get_thread_detail(
        db.get_ref(),
        &session,
        path.into_inner(),
        query.page.unwrap_or(1),
        query.page_size.unwrap_or(5),
    )
    .await
    {
        Ok(thread) => HttpResponse::Ok().json(thread),
        Err(response) => response,
    }
}

#[put("/discussions/threads/{thread_id}")]
pub async fn update_discussion_thread(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    body: web::Json<UpdateDiscussionThread>,
) -> impl Responder {
    match discussion_service::update_thread(
        db.get_ref(),
        &session,
        path.into_inner(),
        body.into_inner(),
    )
    .await
    {
        Ok(thread) => HttpResponse::Ok().json(thread),
        Err(response) => response,
    }
}

#[post("/discussions/threads/{thread_id}/close")]
pub async fn close_discussion_thread(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    match discussion_service::close_thread(db.get_ref(), &session, path.into_inner()).await {
        Ok(thread) => HttpResponse::Ok().json(thread),
        Err(response) => response,
    }
}

#[post("/discussions/threads/{thread_id}/hide")]
pub async fn hide_discussion_thread(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    match discussion_service::hide_thread(db.get_ref(), &session, path.into_inner()).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(response) => response,
    }
}

#[post("/discussions/threads/{thread_id}/replies")]
pub async fn create_discussion_reply(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    body: web::Json<CreateDiscussionReply>,
) -> impl Responder {
    match discussion_service::create_reply(
        db.get_ref(),
        &session,
        path.into_inner(),
        body.into_inner(),
    )
    .await
    {
        Ok(reply) => HttpResponse::Ok().json(reply),
        Err(response) => response,
    }
}

#[delete("/discussions/replies/{reply_id}")]
pub async fn delete_discussion_reply(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    match discussion_service::delete_reply(db.get_ref(), &session, path.into_inner()).await {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(response) => response,
    }
}
