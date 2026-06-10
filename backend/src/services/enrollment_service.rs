use actix_session::Session;
use actix_web::HttpResponse;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

use crate::entity::{courses, enrollments};
use crate::services::auth_helpers::get_user_id;

pub async fn enroll_free_course(
    db: &DatabaseConnection,
    session: &Session,
    course_id: i32,
) -> HttpResponse {
    let user_id = match get_user_id(session) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let course = match courses::Entity::find_by_id(course_id).one(db).await {
        Ok(Some(course)) => course,
        Ok(None) => return HttpResponse::NotFound().body("Course not found in database"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding course: {}", err));
        }
    };

    if course.is_paid.unwrap_or(false) {
        return HttpResponse::BadRequest().body("This is a paid course. Please use checkout.");
    }

    match enrollments::Entity::find()
        .filter(enrollments::Column::UserId.eq(user_id))
        .filter(enrollments::Column::CourseId.eq(course_id))
        .one(db)
        .await
    {
        Ok(Some(_)) => return HttpResponse::BadRequest().body("User is already enrolled in this course"),
        Ok(None) => {}
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error checking enrollment: {}", err));
        }
    }

    let new_enrollment = enrollments::ActiveModel {
        user_id: Set(user_id),
        course_id: Set(course_id),
        ..Default::default()
    };

    match new_enrollment.insert(db).await {
        Ok(_) => HttpResponse::Ok().body("Enrolled in course successfully"),
        Err(err) => {
            HttpResponse::InternalServerError().body(format!("Database error creating enrollment: {}", err))
        }
    }
}
