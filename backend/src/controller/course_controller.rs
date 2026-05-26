use std::f64::consts::PI;

use actix_web::{HttpResponse, HttpServer, Responder, get, web, post, put, delete};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, Set, ActiveModelTrait};
use sea_orm::sea_query::Expr;
use sea_orm::sea_query::extension::postgres::PgExpr;
use crate::entity::courses::{self, CourseStatus}; 
use crate::models::course::{CreateCourse, CourseQuery, UpdateCourse};

#[get("/allcourses")]
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

#[get("/course")]
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

#[put("/course/{course_id}")]
pub async fn update_course(
    db:web::Data<DatabaseConnection>,
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
            let mut active :courses::ActiveModel = course.into();

            if let Some(name) = data.name {
                active.name = Set(Some(name));
            }
            if let Some(instructor_id) = data.instructor_id {
                active.instructor_id = Set(Some(instructor_id));
            }
            if let Some(org_id) = data.org_id {
                active.org_id = Set(Some(org_id));
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

            if let Some(price_cents) = data.price_cents {
                active.price_cents = Set(Some(price_cents));
            }
            if let Some(currency) = data.currency {
                active.currency = Set(Some(currency));
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


#[post("/course")]
pub async fn create_course(
    db: web::Data<DatabaseConnection>,
    body: web::Json<CreateCourse>
) -> impl Responder {

    let data = body.into_inner();

    let course = courses::ActiveModel {
        name: Set(Some(data.name)),
        instructor_id: Set(Some(data.instructor_id)),
        org_id: Set(Some(data.org_id)),

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

        price_cents: Set(Some(data.price_cents)),
        currency: Set(Some(data.currency)),
        is_paid: Set(Some(data.is_paid)),

        ..Default::default()
    };

    match course.insert(db.get_ref()).await {

        Ok(_) => HttpResponse::Ok()
            .body("New course created successfully!"),

        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Insert error: {}", err))
    }
}

#[delete("/course/{course_id}")]
pub async fn delete_course(
    db:web::Data<DatabaseConnection>, 
    path:web::Path<i32>
)-> impl Responder {
    let course_id = path.into_inner();
    let existing = courses::Entity::find_by_id(course_id)
    .one(db.get_ref())
    .await;

    match existing {
        Ok(Some(course)) => {
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