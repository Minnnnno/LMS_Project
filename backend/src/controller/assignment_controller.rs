use actix_web::{HttpResponse, HttpServer, Responder, get, web, post, put, delete};
use lettre::transport::smtp::commands::Data;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, Set, ActiveModelTrait};
use crate::entity::assignments;
use crate::models::assignment::{
    UpdateAssignment,
    CreateAssignment,
};
#[get("/assignment")]
pub async fn get_assignment(
    db: web::Data<DatabaseConnection>
) -> impl Responder {
    let result = assignments::Entity::find()
    .all(db.get_ref())
    .await;
    match result {
        Ok(assignment) => {
            if assignment.is_empty(){
                HttpResponse::NotFound()
                .body("No assignments found")
            }else{
                HttpResponse::Ok().json(assignment)
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

#[get("/assignment/{course_id}")]
pub async fn get_assignment_by_course_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>
) -> impl Responder {
    let course_id = path.into_inner(); 
    let result = assignments::Entity::find()
    .filter(assignments::Column::CourseId.eq(course_id))
    .all(db.get_ref())
    .await;
    match result {
        Ok(assignment) => {
            HttpResponse::Ok().json(assignment)
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

#[put("/assignment/{assignment_id}")]
pub async fn update_assignment(
    db:web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    body: web::Json<UpdateAssignment>
) -> impl Responder {
    let assignment_id = path.into_inner();
    let data = body.into_inner();
    let existing = assignments::Entity::find_by_id(assignment_id)
    .one(db.get_ref())
    .await;

    match existing {
        Ok(Some(assignment)) => {
            let mut active :assignments::ActiveModel = assignment.into();

            if let Some(course_id) = data.course_id {
                active.course_id = Set(course_id);
            }
            if let Some(title) = data.title {
                active.title = Set(title);
            }
            if let Some(description) = data.description {
                active.description= Set(description);
            }
            if let Some(due_date) = data.due_date {
                active.due_date = Set(due_date);
            }
            if let Some(max_score) = data.max_score {
                active.max_score = Set(max_score);
            }

            match active.update(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok()
                .body(format!("Assignment with id {} updated!", assignment_id)),
                Err(err) => HttpResponse::InternalServerError()
                .body(format!("Update error: {}", err))
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Assignment not found"), 
        Err(err) => HttpResponse::InternalServerError()
        .body(format!("Database error: {}", err))
    }
}

#[post("/assignment")]
pub async fn create_assignment(
    db:web::Data<DatabaseConnection>, 
    body:web::Json<CreateAssignment>
) -> impl Responder {
    let data = body.into_inner(); 
    let assignment = assignments::ActiveModel{
        course_id: Set(data.course_id),
        title: Set(data.title),
        description: Set(data.description),
        due_date: Set(data.due_date), 
        max_score: Set(data.max_score),
        ..Default::default()
    };
    match assignment.insert(db.get_ref()).await {
        Ok(_) => HttpResponse::Ok()
        .body("New assignment created successfully!"), 
        Err(err) => HttpResponse::InternalServerError()
        .body(format!("Insert error: {}", err))
    }
}

#[delete("/assignment/{assignment_id}")]
pub async fn delete_assignment(
    db:web::Data<DatabaseConnection>, 
    path:web::Path<i32>
)-> impl Responder {
    let assignment_id = path.into_inner();
    let existing = assignments::Entity::find_by_id(assignment_id)
    .one(db.get_ref())
    .await;

    match existing {
        Ok(Some(assignment)) => {
            let active_model:assignments::ActiveModel = assignment.into();
            match active_model.delete(db.get_ref()).await {
                Ok(_) => {
                    HttpResponse::Ok()
                    .body("Assignment deleted!")
                }
                Err(err) => {
                    HttpResponse::InternalServerError()
                    .body(format!("Delete error: {}", err))
                }
            }
        }
        Ok(None) => {
            HttpResponse::NotFound()
            .body("Assignment not found!")
        }
        Err(err) => {
            HttpResponse::InternalServerError()
            .body(format!("Delete error {}", err))
        }
    }
}