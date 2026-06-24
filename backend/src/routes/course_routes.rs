use actix_web::web;

use crate::controller::course_controller::{
    create_course, delete_course, get_course_completion_roster, get_course_overview, get_courses,
    get_my_courses, get_my_courses_assessments_overview, get_my_courses_assignments_overview,
    get_my_courses_completion_overview, get_my_courses_content_overview,
    get_my_courses_progress_overview, get_organisation_courses, mark_course_manual_completion,
    search_course, undo_course_manual_completion, update_course,
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(get_courses);
    cfg.service(get_organisation_courses);
    cfg.service(get_my_courses);
    cfg.service(get_my_courses_progress_overview);
    cfg.service(get_my_courses_completion_overview);
    cfg.service(get_my_courses_assignments_overview);
    cfg.service(get_my_courses_content_overview);
    cfg.service(get_my_courses_assessments_overview);
    cfg.service(get_course_completion_roster);
    cfg.service(mark_course_manual_completion);
    cfg.service(undo_course_manual_completion);
    cfg.service(get_course_overview);
    cfg.service(search_course);
    cfg.service(update_course);
    cfg.service(create_course);
    cfg.service(delete_course);
}
