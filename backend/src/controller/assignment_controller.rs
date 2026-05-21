use actix_web::{get, web, HttpResponse, HttpServer, Responder};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait};
use crate::entity::assignments;

#[get("/assignment/{course_id}")]
pub async fn get_assignment(
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
            if assignment.is_empty() {
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