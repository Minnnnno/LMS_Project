use crate::entity::courses::{self, CourseStatus};
use crate::entity::enrollments;
use crate::models::course::{CourseQuery, CreateCourse, UpdateCourse};
use crate::services::course_service::{
    can_manage_course,
    can_view_course,
    get_instructor_course_ids_for_session,
    get_instructor_courses_for_session,
    get_organisation_courses_for_session,
    get_session_user_org_id,
    has_role,
    is_instructor_course_limited,
    normalize_course_visibility,
    price_to_cents,
};
use crate::services::module_progress_service;
use actix_session::Session;
use actix_web::{HttpResponse, Responder, delete, get, post, put, web};
use sea_orm::sea_query::Expr;
use sea_orm::sea_query::extension::postgres::PgExpr;
use sea_orm::{ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter, Set};

async fn accessible_course_condition(
    db: &DatabaseConnection,
    session: &Session,
) -> Result<Option<Condition>, HttpResponse> {
    if has_role(session, "LMS Admin") {
        return Ok(None);
    }

    let user_org_id = match session.get::<i32>("user_id") {
        Ok(Some(_)) => match get_session_user_org_id(db, session).await {
            Ok(org_id) => org_id,
            Err(response) => return Err(response),
        },
        Ok(None) => None,
        Err(err) => {
            return Err(HttpResponse::InternalServerError()
                .body(format!("Session error: {}", err)));
        }
    };

    let mut condition = Condition::any().add(courses::Column::Visibility.eq("public"));

    if let Some(org_id) = user_org_id {
        condition = condition.add(
            Condition::all()
                .add(courses::Column::Visibility.eq("private"))
                .add(courses::Column::OrgId.eq(org_id)),
        );
    }

    Ok(Some(condition))
}

#[get("/courses")]
pub async fn get_courses(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    if is_instructor_course_limited(&session) {
        return match get_instructor_courses_for_session(db.get_ref(), &session).await {
            Ok(courses) => HttpResponse::Ok().json(courses),
            Err(response) => response,
        };
    }

    let mut query = courses::Entity::find();

    if let Some(condition) = match accessible_course_condition(db.get_ref(), &session).await {
        Ok(condition) => condition,
        Err(response) => return response,
    } {
        query = query.filter(condition);
    }

    let result = query.all(db.get_ref()).await;
    match result {
        Ok(course) => {
            if course.is_empty() {
                HttpResponse::NotFound().body("No courses found")
            } else {
                HttpResponse::Ok().json(course)
            }
        }
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

#[get("/my-courses")]
pub async fn get_my_courses(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    if is_instructor_course_limited(&session) {
        return match get_instructor_courses_for_session(db.get_ref(), &session).await {
            Ok(courses) => HttpResponse::Ok().json(courses),
            Err(response) => response,
        };
    }

    let user_id = match session.get::<i32>("user_id") {
        Ok(Some(id)) => id,
        Ok(None) => {
            return HttpResponse::Unauthorized().body("User not logged in");
        }
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Session error: {}", err));
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
    match get_organisation_courses_for_session(db.get_ref(), &session).await {
        Ok(courses) => HttpResponse::Ok().json(courses),
        Err(response) => response,
    }
}

#[get("/course/{course_id}")]
pub async fn get_course_by_course_id(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>
) -> impl Responder {
    let course_id = path.into_inner();
    let result = courses::Entity::find_by_id(course_id)
        .one(db.get_ref())
        .await;
    match result {
        Ok(course) => {
            if let Some(course) = course {
                if is_instructor_course_limited(&session) {
                    match can_manage_course(db.get_ref(), &session, &course).await {
                        Ok(true) => {}
                        Ok(false) => {
                            return HttpResponse::Forbidden()
                                .body("You can only view courses assigned to you");
                        }
                        Err(response) => return response,
                    }
                }

                match can_view_course(db.get_ref(), &session, &course).await {
                    Ok(true) => {}
                    Ok(false) => {
                        return HttpResponse::Forbidden()
                            .body("This course is private to its organisation");
                    }
                    Err(response) => return response,
                }

                HttpResponse::Ok().json(course)
            } else {
                HttpResponse::NotFound().body("Course not found")
            }
        }
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

#[get("/courses/{course_id}/manage-access")]
pub async fn get_course_manage_access(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    let course_id = path.into_inner();
    let course = match courses::Entity::find_by_id(course_id)
        .one(db.get_ref())
        .await
    {
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

#[get("/courses/{course_id}/progress")]
pub async fn get_course_progress(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    match module_progress_service::get_course_progress(
        db.get_ref(),
        &session,
        path.into_inner(),
    )
    .await
    {
        Ok(progress) => HttpResponse::Ok().json(progress),
        Err(response) => response,
    }
}

#[get("/courses/{course_id}/module-progress")]
pub async fn get_course_module_progress(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    match module_progress_service::get_course_module_progress(
        db.get_ref(),
        &session,
        path.into_inner(),
    )
    .await
    {
        Ok(progress) => HttpResponse::Ok().json(progress),
        Err(response) => response,
    }
}

#[get("/courses/search")]
pub async fn search_course(
    db: web::Data<DatabaseConnection>,
    session: Session,
    query: web::Query<CourseQuery>,
) -> impl Responder {
    let mut db_query = courses::Entity::find();

    if let Some(name) = &query.name {
        db_query = db_query.filter(Expr::col(courses::Column::Name).ilike(format!("%{}%", name)));
    }

    if let Some(instructor_id) = &query.instructor_id {
        db_query = db_query.filter(courses::Column::InstructorId.eq(*instructor_id))
    }

    if let Some(min_price) = query.min_price {
        db_query = db_query.filter(courses::Column::PriceCents.gte(min_price));
    }

    if let Some(max_price) = query.max_price {
        db_query = db_query.filter(courses::Column::PriceCents.lte(max_price));
    }
    if let Some(course_id) = query.course_id {
        db_query = db_query.filter(courses::Column::CourseId.eq(course_id))
    }

    if is_instructor_course_limited(&session) {
        let course_ids = match get_instructor_course_ids_for_session(db.get_ref(), &session).await {
            Ok(course_ids) => course_ids,
            Err(response) => return response,
        };

        if course_ids.is_empty() {
            return HttpResponse::Ok().json(Vec::<courses::Model>::new());
        }

        db_query = db_query.filter(courses::Column::CourseId.is_in(course_ids));
    } else if let Some(condition) = match accessible_course_condition(db.get_ref(), &session).await {
        Ok(condition) => condition,
        Err(response) => return response,
    } {
        db_query = db_query.filter(condition);
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

        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

#[put("/courses/{course_id}")]
pub async fn update_course(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    body: web::Json<UpdateCourse>,
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

            let updated_price_cents = match data.price {
                Some(price) => match price_to_cents(price) {
                    Ok(price_cents) => price_cents,
                    Err(response) => return response,
                },
                None => course.price_cents.unwrap_or(0),
            };
            let updated_is_paid = data.is_paid.unwrap_or(course.is_paid.unwrap_or(false));

            if updated_is_paid && updated_price_cents <= 0 {
                return HttpResponse::BadRequest()
                    .body("Paid courses must have a price greater than zero");
            }

            let mut active: courses::ActiveModel = course.into();

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
                        return HttpResponse::Forbidden().body(
                            "Organisation Admin cannot move courses outside their organisation",
                        );
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
                        return HttpResponse::BadRequest().body("Invalid course status");
                    }
                };

                active.status = Set(course_status);
            }

            if data.price.is_some() {
                active.price_cents = Set(Some(updated_price_cents));
            }
            if let Some(currency) = data.currency {
                active.currency = Set(Some(currency));
            }
            if let Some(is_paid) = data.is_paid {
                active.is_paid = Set(Some(is_paid));
            }

            if let Some(description) = data.description {
                active.description = Set(Some(description));
            }
            if let Some(background_image_url) = data.background_image_url {
                active.background_image_url = Set(Some(background_image_url));
            }
            if let Some(visibility) = data.visibility {
                active.visibility = Set(match normalize_course_visibility(Some(visibility)) {
                    Ok(visibility) => visibility,
                    Err(response) => return response,
                });
            }

            match active.update(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok().body(format!("Course with id {} updated!", course_id)),
                Err(err) => {
                    HttpResponse::InternalServerError().body(format!("Update error: {}", err))
                }
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Course not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

#[post("/courses")]
pub async fn create_course(
    db: web::Data<DatabaseConnection>,
    session: Session,
    body: web::Json<CreateCourse>,
) -> impl Responder {
    let data = body.into_inner();

    if !has_role(&session, "Organisation Admin") {
        return HttpResponse::Forbidden()
            .body("Organisation Admin role required to create courses");
    }

    let session_user_id = match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => user_id,
        Ok(None) => return HttpResponse::Unauthorized().body("User not logged in"),
        Err(err) => {
            return HttpResponse::InternalServerError().body(format!("Session error: {}", err));
        }
    };

    let org_id = match get_session_user_org_id(db.get_ref(), &session).await {
        Ok(Some(user_org_id)) => user_org_id,
        Ok(None) => {
            return HttpResponse::Forbidden()
                .body("Organisation Admin is not assigned to an organisation");
        }
        Err(response) => return response,
    };

    let price_cents = match price_to_cents(data.price) {
        Ok(price_cents) => price_cents,
        Err(response) => return response,
    };

    if data.is_paid && price_cents <= 0 {
        return HttpResponse::BadRequest()
            .body("Paid courses must have a price greater than zero");
    }

    let visibility = match normalize_course_visibility(data.visibility) {
        Ok(visibility) => visibility,
        Err(response) => return response,
    };

    let course = courses::ActiveModel {
        name: Set(Some(data.name)),
        instructor_id: Set(Some(data.instructor_id.unwrap_or(session_user_id))),
        org_id: Set(Some(org_id)),

        status: Set(match data.status.as_str() {
            "draft" => CourseStatus::Draft,
            "published" => CourseStatus::Published,
            "archived" => CourseStatus::Archived,

            _ => {
                return HttpResponse::BadRequest().body("Invalid course status");
            }
        }),

        price_cents: Set(Some(price_cents)),
        currency: Set(Some(data.currency)),
        is_paid: Set(Some(data.is_paid)),
        description: Set(data.description),
        background_image_url: Set(data.background_image_url),
        visibility: Set(visibility),

        ..Default::default()
    };

    match course.insert(db.get_ref()).await {
        Ok(_) => HttpResponse::Ok().body("New course created successfully!"),

        Err(err) => HttpResponse::InternalServerError().body(format!("Insert error: {}", err)),
    }
}

#[delete("/courses/{course_id}")]
pub async fn delete_course(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
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

            let active_model: courses::ActiveModel = course.into();
            match active_model.delete(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok().body("Course deleted!"),
                Err(err) => {
                    HttpResponse::InternalServerError().body(format!("Delete error: {}", err))
                }
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Course not found!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Delete error {}", err)),
    }
}
