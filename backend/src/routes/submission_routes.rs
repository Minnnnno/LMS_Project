use actix_web::web;

use crate::controller::submission_controller::{
    clear_submission_grade, create_submission, grade_submission, list_assignment_submissions,
    list_my_submissions,
};

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(create_submission);
    cfg.service(list_my_submissions);
    cfg.service(list_assignment_submissions);
    cfg.service(grade_submission);
    cfg.service(clear_submission_grade);
}
