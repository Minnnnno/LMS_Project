use actix_web::web;

use crate::controller::discussion_controller::{
    close_discussion_thread, create_discussion_reply, create_discussion_thread,
    create_discussion_topic, delete_discussion_reply, delete_discussion_topic,
    get_discussion_thread, hide_discussion_thread, list_course_discussion_topics,
    list_discussion_threads, list_discussion_topics, update_discussion_thread,
    update_discussion_topic,
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(list_discussion_topics);
    cfg.service(list_course_discussion_topics);
    cfg.service(create_discussion_topic);
    cfg.service(update_discussion_topic);
    cfg.service(delete_discussion_topic);
    cfg.service(list_discussion_threads);
    cfg.service(create_discussion_thread);
    cfg.service(get_discussion_thread);
    cfg.service(update_discussion_thread);
    cfg.service(close_discussion_thread);
    cfg.service(hide_discussion_thread);
    cfg.service(create_discussion_reply);
    cfg.service(delete_discussion_reply);
}
