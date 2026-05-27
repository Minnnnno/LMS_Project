use std::f64::consts::PI;

use actix_web::{HttpResponse, HttpServer, Responder, get, web, post, put, delete};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, Set, ActiveModelTrait};
use sea_orm::sea_query::Expr;
use sea_orm::sea_query::extension::postgres::PgExpr;
use crate::entity::modules::{self, Entity as Modules}; 
use crate::models::modules::{
    CreateModules,
    ModulesQuery,
    UpdateModules,
};

#[get("/allmodules")]
pub async fn get_modules(
    db: web::Data<DatabaseConnection>
) -> impl Responder {
    let result = modules::Entity::find()
    .all(db.get_ref())
    .await;
    match result {
        Ok(modules) => {
            if modules.is_empty(){
                HttpResponse::NotFound()
                .body("No modules found")
            }else{
                HttpResponse::Ok().json(modules)
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err))
    }
  }

#[get("/module/{course_id}")]
pub async fn get_modules_by_course_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>
) -> impl Responder {
    let course_id = path.into_inner(); 
    let result = modules::Entity::find()
    .filter(modules::Column::CourseId.eq(course_id))
    .all(db.get_ref())
    .await;
    match result {
        Ok(modules) => {
            if modules.is_empty() {
                HttpResponse::NotFound()
                .body("No modules found")
            
            }else{
                HttpResponse::Ok().json(modules)
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

#[put("/modules/{module_id}")]
pub async fn update_module(
    db:web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    body: web::Json<UpdateModules>
) -> impl Responder {
    let module_id = path.into_inner();
    let data = body.into_inner();
    let existing = modules::Entity::find_by_id(module_id)
    .one(db.get_ref())
    .await;

    match existing {
        Ok(Some(module)) => {
            let mut active :modules::ActiveModel = module.into();

            if let Some(module_id) = data.module_id {
                active.module_id = Set(module_id);
            }
            if let Some(course_id) = data.course_id {
                active.course_id = Set(course_id);
            }
            if let Some(title) = data.title {
                active.title = Set(title);
            }

            if let Some(position) = data.position {
                active.position = Set(position);
            }

            match active.update(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok()
                .body(format!("module with id {} updated!", module_id)),
                Err(err) => HttpResponse::InternalServerError()
                .body(format!("Update error: {}", err))
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Module not found"), 
        Err(err) => HttpResponse::InternalServerError()
        .body(format!("Database error: {}", err))
    }
}

#[post("/modules")]
pub async fn create_module(
    db: web::Data<DatabaseConnection>,
    body: web::Json<CreateModules>
) -> impl Responder {

    let data = body.into_inner();

    let module = modules::ActiveModel {
        course_id: Set(data.course_id),
        title: Set(data.title),
        position: Set(data.position),
        ..Default::default()
    };

    match module.insert(db.get_ref()).await {

        Ok(_) => HttpResponse::Ok()
            .body("New module created successfully!"),

        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Insert error: {}", err))
    }
}

#[delete("/module/{module_id}")]
pub async fn delete_module(
    db:web::Data<DatabaseConnection>, 
    path:web::Path<i32>
)-> impl Responder {
    let module_id = path.into_inner();
    let existing = modules::Entity::find_by_id(module_id)
    .one(db.get_ref())
    .await;

    match existing {
        Ok(Some(module)) => {
            let active_model:modules::ActiveModel = module.into();
            match active_model.delete(db.get_ref()).await {
                Ok(_) => {
                    HttpResponse::Ok()
                    .body("Module deleted!")
                }
                Err(err) => {
                    HttpResponse::InternalServerError()
                    .body(format!("Delete error: {}", err))
                }
            }
        }
        Ok(None) => {
            HttpResponse::NotFound()
            .body("Module not found!")
        }
        Err(err) => {
            HttpResponse::InternalServerError()
            .body(format!("Delete error {}", err))
        }
    }
}