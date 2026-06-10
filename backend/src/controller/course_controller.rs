use std::f64::consts::PI;

use actix_session::Session;
use actix_web::{HttpResponse, HttpServer, Responder, get, web, post, put, delete};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, Set, ActiveModelTrait};
use sea_orm::sea_query::Expr;
use sea_orm::sea_query::extension::postgres::PgExpr;
use rust_decimal::prelude::ToPrimitive;
use crate::entity::courses::{self, CourseStatus};
use crate::entity::{enrollments, users};
use crate::models::course::{CreateCourse, CourseQuery, UpdateCourse};

fn price_to_cents(price: rust_decimal::Decimal) -> Result<i32, HttpResponse> {
    if price.is_sign_negative() {
        return Err(HttpResponse::BadRequest().body("Price cannot be negative"));
    }

    let cents = (price * rust_decimal::Decimal::new(100, 0))
        .round_dp(0)
        .to_i32()
        .ok_or_else(|| HttpResponse::BadRequest().body("Invalid price"))?;

    Ok(cents)
}

fn get_role_names(session: &Session) -> Vec<String> {
    session
        .get::<Vec<String>>("role_names")
        .ok()
        .flatten()
        .unwrap_or_default()
}

fn has_role(session: &Session, role_name: &str) -> bool {
    get_role_names(session).iter().any(|role| role == role_name)
}

async fn can_manage_course(
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

    Ok(user.org_id.is_some() && user.org_id == course.org_id)
}

async fn get_session_user_org_id(
    db: &DatabaseConnection,
    session: &Session,
) -> Result<Option<i32>, HttpResponse> {
    let user_id = match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => user_id,
        Ok(None) => return Err(HttpResponse::Unauthorized().body("User not logged in")),
        Err(err) => {
            return Err(HttpResponse::InternalServerError()
                .body(format!("Session error: {}", err)));
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

#[get("/courses")]
pub async fn get_courses(
    db: web::Data<DatabaseConnection>
) -> impl Responder {
    let result = courses::Entity::find()
    .all(db.get_ref())
    .await;
    match result {
        Ok(course) => {
            if course.is_empty(){
                HttpResponse::NotFound()
                .body("No courses found")
            }else{
                HttpResponse::Ok().json(course)
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err))
    }
  }

#[get("/my-courses")]
pub async fn get_my_courses(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    let user_id = match session.get::<i32>("user_id") {
        Ok(Some(id)) => id,
        Ok(None) => {
            return HttpResponse::Unauthorized()
                .body("User not logged in");
        }
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Session error: {}", err));
        }
    };

    let enrollment_rows = match enrollments::Entity::find()
        .filter(enrollments::Column::UserId.eq(user_id))
        .all(db.get_ref())
        .await
    {
        Ok(rows) => rows,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding enrollments: {}", err));
        }
    };

    let course_ids: Vec<i32> = enrollment_rows
        .into_iter()
        .map(|enrollment| enrollment.course_id)
        .collect();

    if course_ids.is_empty() {
        return HttpResponse::Ok().json(Vec::<courses::Model>::new());
    }

    match courses::Entity::find()
        .filter(courses::Column::CourseId.is_in(course_ids))
        .all(db.get_ref())
        .await
    {
        Ok(courses) => HttpResponse::Ok().json(courses),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error finding courses: {}", err)),
    }
}

#[get("/courses/organisation")]
pub async fn get_organisation_courses(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    if !has_role(&session, "Organisation Admin") && !has_role(&session, "LMS Admin") {
        return HttpResponse::Forbidden().body("Organisation Admin role required");
    }

    if has_role(&session, "LMS Admin") {
        return match courses::Entity::find().all(db.get_ref()).await {
            Ok(courses) => HttpResponse::Ok().json(courses),
            Err(err) => HttpResponse::InternalServerError()
                .body(format!("Database error finding courses: {}", err)),
        };
    }

    let user_id = match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => user_id,
        Ok(None) => return HttpResponse::Unauthorized().body("User not logged in"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Session error: {}", err));
        }
    };

    let user = match users::Entity::find_by_id(user_id).one(db.get_ref()).await {
        Ok(Some(user)) => user,
        Ok(None) => return HttpResponse::NotFound().body("User not found"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding user: {}", err));
        }
    };

    let org_id = match user.org_id {
        Some(org_id) => org_id,
        None => {
            return HttpResponse::Forbidden()
                .body("Organisation Admin is not assigned to an organisation");
        }
    };

    match courses::Entity::find()
        .filter(courses::Column::OrgId.eq(org_id))
        .all(db.get_ref())
        .await
    {
        Ok(courses) => HttpResponse::Ok().json(courses),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error finding organisation courses: {}", err)),
    }
}

#[get("/course/{course_id}")]
pub async fn get_course_by_course_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>
) -> impl Responder {
    let course_id = path.into_inner(); 
    let result = courses::Entity::find_by_id(course_id)
    .one(db.get_ref())
    .await;
    match result {
        Ok(course) => {
            if let Some(course) = course {
                HttpResponse::Ok().json(course)
            } else {
                HttpResponse::NotFound().body("Course not found")
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err))
    }
}

#[get("/courses/{course_id}/manage-access")]
pub async fn get_course_manage_access(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    let course_id = path.into_inner();
    let course = match courses::Entity::find_by_id(course_id).one(db.get_ref()).await {
        Ok(Some(course)) => course,
        Ok(None) => return HttpResponse::NotFound().body("Course not found"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error finding course: {}", err));
        }
    };

    match can_manage_course(db.get_ref(), &session, &course).await {
        Ok(can_manage) => HttpResponse::Ok().json(serde_json::json!({
            "can_manage": can_manage
        })),
        Err(response) => response,
    }
}

#[get("/courses/search")]
pub async fn search_course(
    db: web::Data<DatabaseConnection>,
    query: web::Query<CourseQuery>,
) -> impl Responder {
    let mut db_query = courses::Entity::find();

    if let Some(name) = &query.name {
        db_query = db_query.filter(
            Expr::col(courses::Column::Name)
                .ilike(format!("%{}%", name))
        );
    }

    if let Some(instructor_id) = &query.instructor_id {
        db_query = db_query.filter(
            courses::Column::InstructorId.eq(*instructor_id)
        )
    }

    if let Some(min_price) = query.min_price {
        db_query = db_query.filter(
            courses::Column::PriceCents.gte(min_price)
        );
    }

    if let Some(max_price) = query.max_price {
        db_query = db_query.filter(
            courses::Column::PriceCents.lte(max_price)
        );
    }
    if let Some(course_id) = query.course_id{
        db_query = db_query.filter(
            courses::Column::CourseId.eq(course_id)
        )
    }

    let result = db_query
        .all(db.get_ref())
        .await;

    match result {
        Ok(course) => {
            if course.is_empty() {
                HttpResponse::NotFound().body("No courses found")
            } else {
                HttpResponse::Ok().json(course)
            }
        }

        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

#[put("/courses/{course_id}")]
pub async fn update_course(
    db:web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    body: web::Json<UpdateCourse>
) -> impl Responder {
    let course_id = path.into_inner();
    let data = body.into_inner();
    let existing = courses::Entity::find_by_id(course_id)
    .one(db.get_ref())
    .await;

    match existing {
        Ok(Some(course)) => {
            match can_manage_course(db.get_ref(), &session, &course).await {
                Ok(true) => {}
                Ok(false) => {
                    return HttpResponse::Forbidden()
                        .body("You can only update courses under your organisation");
                }
                Err(response) => return response,
            }

            let mut active :courses::ActiveModel = course.into();

            if let Some(name) = data.name {
                active.name = Set(Some(name));
            }
            if let Some(instructor_id) = data.instructor_id {
                active.instructor_id = Set(Some(instructor_id));
            }
            if let Some(org_id) = data.org_id {
                if has_role(&session, "LMS Admin") {
                    active.org_id = Set(Some(org_id));
                } else {
                    let user_org_id = match get_session_user_org_id(db.get_ref(), &session).await {
                        Ok(user_org_id) => user_org_id,
                        Err(response) => return response,
                    };

                    if user_org_id != Some(org_id) {
                        return HttpResponse::Forbidden()
                            .body("Organisation Admin cannot move courses outside their organisation");
                    }

                    active.org_id = Set(Some(org_id));
                }
            }
            if let Some(status) = data.status {

            let course_status = match status.as_str() {
                "draft" => CourseStatus::Draft,
                "published" => CourseStatus::Published,
                "archived" => CourseStatus::Archived,

                _ => {
                    return HttpResponse::BadRequest()
                        .body("Invalid course status");
                }
            };

                active.status = Set(course_status);
            }

            if let Some(price) = data.price {
                active.price_cents = Set(Some(match price_to_cents(price) {
                    Ok(price_cents) => price_cents,
                    Err(response) => return response,
                }));
            }
            if let Some(currency) = data.currency {
                active.currency = Set(Some(currency));
            }
            if let Some(is_paid) = data.is_paid {
                active.is_paid = Set(Some(is_paid));
            }

if          let Some(description) = data.description {
                active.description = Set(Some(description));
            }
            if let Some(background_image_url) = data.background_image_url {
                active.background_image_url = Set(Some(background_image_url));
            }

            match active.update(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok()
                .body(format!("Course with id {} updated!", course_id)),
                Err(err) => HttpResponse::InternalServerError()
                .body(format!("Update error: {}", err))
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Course not found"), 
        Err(err) => HttpResponse::InternalServerError()
        .body(format!("Database error: {}", err))
    }
}


#[post("/courses")]
pub async fn create_course(
    db: web::Data<DatabaseConnection>,
    session: Session,
    body: web::Json<CreateCourse>
) -> impl Responder {

    let data = body.into_inner();

    if !has_role(&session, "LMS Admin") && !has_role(&session, "Organisation Admin") {
        return HttpResponse::Forbidden().body("Admin role required to create courses");
    }

    let org_id = if has_role(&session, "LMS Admin") {
        data.org_id
    } else {
        match get_session_user_org_id(db.get_ref(), &session).await {
            Ok(Some(user_org_id)) if user_org_id == data.org_id => user_org_id,
            Ok(Some(_)) => {
                return HttpResponse::Forbidden()
                    .body("Organisation Admin can only create courses under their organisation");
            }
            Ok(None) => {
                return HttpResponse::Forbidden()
                    .body("Organisation Admin is not assigned to an organisation");
            }
            Err(response) => return response,
        }
    };

    let course = courses::ActiveModel {
        name: Set(Some(data.name)),
        instructor_id: Set(Some(data.instructor_id)),
        org_id: Set(Some(org_id)),

        status: Set(
            match data.status.as_str() {
                "draft" => CourseStatus::Draft,
                "published" => CourseStatus::Published,
                "archived" => CourseStatus::Archived,

                _ => {
                    return HttpResponse::BadRequest()
                        .body("Invalid course status");
                }
            }
        ),

        price_cents: Set(Some(match price_to_cents(data.price) {
            Ok(price_cents) => price_cents,
            Err(response) => return response,
        })),
        currency: Set(Some(data.currency)),
        is_paid: Set(Some(data.is_paid)),
        description: Set(data.description),
        background_image_url: Set(data.background_image_url),

        ..Default::default()
    };

    match course.insert(db.get_ref()).await {

        Ok(_) => HttpResponse::Ok()
            .body("New course created successfully!"),

        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Insert error: {}", err))
    }
}

#[delete("/courses/{course_id}")]
pub async fn delete_course(
    db:web::Data<DatabaseConnection>,
    session: Session,
    path:web::Path<i32>
)-> impl Responder {
    let course_id = path.into_inner();
    let existing = courses::Entity::find_by_id(course_id)
    .one(db.get_ref())
    .await;

    match existing {
        Ok(Some(course)) => {
            match can_manage_course(db.get_ref(), &session, &course).await {
                Ok(true) => {}
                Ok(false) => {
                    return HttpResponse::Forbidden()
                        .body("You can only delete courses under your organisation");
                }
                Err(response) => return response,
            }

            let active_model:courses::ActiveModel = course.into();
            match active_model.delete(db.get_ref()).await {
                Ok(_) => {
                    HttpResponse::Ok()
                    .body("Course deleted!")
                }
                Err(err) => {
                    HttpResponse::InternalServerError()
                    .body(format!("Delete error: {}", err))
                }
            }
        }
        Ok(None) => {
            HttpResponse::NotFound()
            .body("Course not found!")
        }
        Err(err) => {
            HttpResponse::InternalServerError()
            .body(format!("Delete error {}", err))
        }
    }
}
