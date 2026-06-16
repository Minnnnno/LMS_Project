use actix_session::Session;
use actix_web::HttpResponse;
use rust_decimal::prelude::ToPrimitive;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use crate::entity::{course_instructors, courses, users};

pub fn get_role_names(session: &Session) -> Vec<String> {
    session
        .get::<Vec<String>>("role_names")
        .ok()
        .flatten()
        .unwrap_or_default()
}

pub fn has_role(session: &Session, role_name: &str) -> bool {
    get_role_names(session).iter().any(|role| role == role_name)
}

pub fn is_instructor_course_limited(session: &Session) -> bool {
    has_role(session, "Instructor")
        && !has_role(session, "Organisation Admin")
        && !has_role(session, "LMS Admin")
}

pub fn price_to_cents(price: rust_decimal::Decimal) -> Result<i32, HttpResponse> {
    if price.is_sign_negative() {
        return Err(HttpResponse::BadRequest().body("Price cannot be negative"));
    }

    (price * rust_decimal::Decimal::new(100, 0))
        .round_dp(0)
        .to_i32()
        .ok_or_else(|| HttpResponse::BadRequest().body("Invalid price"))
}

pub fn normalize_course_visibility(visibility: Option<String>) -> Result<String, HttpResponse> {
    let visibility = visibility.unwrap_or_else(|| "public".to_string()).to_lowercase();

    match visibility.as_str() {
        "public" | "private" => Ok(visibility),
        _ => Err(HttpResponse::BadRequest().body("Invalid course visibility")),
    }
}

pub async fn get_session_user_org_id(
    db: &DatabaseConnection,
    session: &Session,
) -> Result<Option<i32>, HttpResponse> {
    let user_id = match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => user_id,
        Ok(None) => return Err(HttpResponse::Unauthorized().body("User not logged in")),
        Err(err) => {
            return Err(HttpResponse::InternalServerError().body(format!("Session error: {}", err)));
        }
    };

    users::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding user: {}", err))
        })?
        .map(|user| user.org_id)
        .ok_or_else(|| HttpResponse::NotFound().body("User not found"))
}

pub async fn get_session_user(
    db: &DatabaseConnection,
    session: &Session,
) -> Result<users::Model, HttpResponse> {
    let user_id = match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => user_id,
        Ok(None) => return Err(HttpResponse::Unauthorized().body("User not logged in")),
        Err(err) => {
            return Err(HttpResponse::InternalServerError().body(format!("Session error: {}", err)));
        }
    };

    users::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding user: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("User not found"))
}

pub async fn can_manage_course(
    db: &DatabaseConnection,
    session: &Session,
    course: &courses::Model,
) -> Result<bool, HttpResponse> {
    if has_role(session, "LMS Admin") {
        return Ok(true);
    }

    let user = get_session_user(db, session).await?;

    if has_role(session, "Organisation Admin")
        && user.org_id.is_some()
        && user.org_id == course.org_id
    {
        return Ok(true);
    }

    if has_role(session, "Instructor") {
        return course_instructors::Entity::find_by_id((course.course_id, user.user_id))
            .one(db)
            .await
            .map(|assignment| assignment.is_some())
            .map_err(|err| {
                HttpResponse::InternalServerError()
                    .body(format!("Database error finding course instructor: {}", err))
            });
    }

    Ok(false)
}

pub async fn can_view_course(
    db: &DatabaseConnection,
    session: &Session,
    course: &courses::Model,
) -> Result<bool, HttpResponse> {
    if course.visibility != "private" {
        return Ok(true);
    }

    if has_role(session, "LMS Admin") {
        return Ok(true);
    }

    let user_id = match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => user_id,
        Ok(None) => return Ok(false),
        Err(err) => {
            return Err(HttpResponse::InternalServerError()
                .body(format!("Session error: {}", err)));
        }
    };

    let user = users::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding user: {}", err))
        })?
        .ok_or_else(|| HttpResponse::NotFound().body("User not found"))?;

    if user.org_id.is_some() && user.org_id == course.org_id {
        return Ok(true);
    }

    can_manage_course(db, session, course).await
}

pub async fn get_instructor_course_ids_for_session(
    db: &DatabaseConnection,
    session: &Session,
) -> Result<Vec<i32>, HttpResponse> {
    let user = get_session_user(db, session).await?;
    let assignments = course_instructors::Entity::find()
        .filter(course_instructors::Column::InstructorId.eq(user.user_id))
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding assigned courses: {}", err))
        })?;

    Ok(assignments
        .into_iter()
        .map(|assignment| assignment.course_id)
        .collect())
}

pub async fn get_instructor_courses_for_session(
    db: &DatabaseConnection,
    session: &Session,
) -> Result<Vec<courses::Model>, HttpResponse> {
    let course_ids = get_instructor_course_ids_for_session(db, session).await?;

    if course_ids.is_empty() {
        return Ok(Vec::new());
    }

    courses::Entity::find()
        .filter(courses::Column::CourseId.is_in(course_ids))
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding assigned courses: {}", err))
        })
}

pub async fn get_organisation_courses_for_session(
    db: &DatabaseConnection,
    session: &Session,
) -> Result<Vec<courses::Model>, HttpResponse> {
    if has_role(session, "LMS Admin") {
        return courses::Entity::find().all(db).await.map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding courses: {}", err))
        });
    }

    if has_role(session, "Organisation Admin") {
        let org_id = get_session_user_org_id(db, session).await?.ok_or_else(|| {
            HttpResponse::Forbidden().body("Organisation Admin is not assigned to an organisation")
        })?;

        return courses::Entity::find()
            .filter(courses::Column::OrgId.eq(org_id))
            .all(db)
            .await
            .map_err(|err| {
                HttpResponse::InternalServerError().body(format!(
                    "Database error finding organisation courses: {}",
                    err
                ))
            });
    }

    if has_role(session, "Instructor") {
        return get_instructor_courses_for_session(db, session).await;
    }

    Err(HttpResponse::Forbidden().body("Course management role required"))
}
