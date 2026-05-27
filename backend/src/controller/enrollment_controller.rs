use actix_session::Session;
use actix_web::{post, web, HttpResponse, Responder};

use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};

use crate::entity::{courses, enrollments};


//helper function to get user id from session
fn get_session_user_id(session: &Session) -> Result<i32, HttpResponse> {
    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => Ok(user_id),             // if user_id is found in session, return it
        Ok(None) => Err(HttpResponse::Unauthorized().body("User not logged in")),  // if user_id is not found in session, return unauthorized
        Err(_) => Err(HttpResponse::InternalServerError().body("Failed to retrieve session")),  // if there is an error retrieving session, return internal server error
    }
}

//enrollment into free course
#[post("/courses/{course_id}/enroll")]

pub async fn enroll_free_course(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    let course_id = path.into_inner();

    //get current user id from session
    let user_id = match get_session_user_id(&session) {
        Ok(id) => id,
        Err(err) => return err,  // if there is an error getting user_id from session, return the error response
    };      //helper function already handles Ok(None) response and Err() response

    //find course from db
    let course = match courses::Entity::find_by_id(course_id)
        .one(db.get_ref())
        .await
    {
        Ok(Some(course)) => course,
        Ok(None) => {
            return HttpResponse::NotFound()
                .body("Course not found in database");
        }
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding course: {}", err));
        }
    };

    //reject paid course enrollment through this route
    if course.is_paid.unwrap_or(false) {
        return HttpResponse::BadRequest()
            .body("This is a paid course. Please use checkout.");
    }


    //check if user is already enrolled in the course
    let existing_enrollment = enrollments::Entity::find()
        .filter(enrollments::Column::UserId.eq(user_id))
        .filter(enrollments::Column::CourseId.eq(course_id))
        .one(db.get_ref())
        .await;

    match existing_enrollment {
        Ok(Some(_)) => {
            return HttpResponse::BadRequest()
                .body("User is already enrolled in this course");
        }
        Ok(None) => {
            // not enrolled yet, continue
        }
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error checking enrollment: {}", err));
        }
    }


    //insert into enrollments table in db

    let new_enrollment = enrollments::ActiveModel {
        user_id: Set(user_id),
        course_id: Set(course_id),
        ..Default::default()
    };

    match new_enrollment.insert(db.get_ref()).await {
        Ok(_) => HttpResponse::Ok().body("Enrolled in course successfully"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error creating enrollment: {}", err)),
    }

}