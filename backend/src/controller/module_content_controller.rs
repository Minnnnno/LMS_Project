use actix_web::{HttpResponse, HttpServer, Responder, get, web, post, put, delete};
use lettre::transport::smtp::commands::Data;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, Set, ActiveModelTrait};
use crate::entity::module_contents;
use crate::models::module_content::{
    UpdateModuleContent,
    CreateModuleContent,
};

#[get("/module-content")]
pub async fn get_module_contents(
    db: web::Data<DatabaseConnection>
) -> impl Responder {
    let result = module_contents::Entity::find()
    .all(db.get_ref())
    .await;
    match result {
        Ok(module_content) => {
            if module_content.is_empty(){
                HttpResponse::NotFound()
                .body("No module content found")
            }else{
                HttpResponse::Ok().json(module_content)
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

#[get("/module-content/{module_id}")]
pub async fn get_module_content_by_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>
) -> impl Responder {
    let module_id = path.into_inner(); 
    let result = module_contents::Entity::find()
    .filter(module_contents::Column::ModuleId.eq(module_id))
    .all(db.get_ref())
    .await;
    match result {
        Ok(module_content) => {
            if module_content.is_empty() {
                HttpResponse::NotFound()
                .body("No module content found")
            
            }else{
                HttpResponse::Ok().json(module_content)
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

#[put("/module-content/{module_content_id}")]
pub async fn update_module_content(
    db:web::Data<DatabaseConnection>,
    path: web::Path<i32>,
    body: web::Json<UpdateModuleContent>
) -> impl Responder {
    let module_content_id = path.into_inner();
    let data = body.into_inner();
    let existing = module_contents::Entity::find_by_id(module_content_id)
    .one(db.get_ref())
    .await;

    match existing {
        Ok(Some(module_content)) => {
            let mut active :module_contents::ActiveModel = module_content.into();

            if let Some(module_id) = data.module_id {
                active.module_id = Set(module_id);
            }
            if let Some(content_type) = data.content_type {
                active.content_type = Set(content_type);
            }
            if let Some(content_category) = data.content_category {
                active.content_category = Set(Some(content_category));
            }
            if let Some(title) = data.title {
                active.title = Set(title);
            }
            if let Some(content_url) = data.content_url {
                active.content_url = Set(Some(content_url));
            }
            if let Some(cloudinary_public_id) = data.cloudinary_public_id {
                active.cloudinary_public_id = Set(Some(cloudinary_public_id));
            }
            if let Some(position) = data.position {
                active.position = Set(position);
            }

            match active.update(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok()
                .body(format!("Module content with id {} updated!", module_content_id)),
                Err(err) => HttpResponse::InternalServerError()
                .body(format!("Update error: {}", err))
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Module content not found"), 
        Err(err) => HttpResponse::InternalServerError()
        .body(format!("Database error: {}", err))
    }
}

#[post("/module-content")]
pub async fn create_module_content(
    db: web::Data<DatabaseConnection>,
    body: web::Json<CreateModuleContent>,
) -> impl Responder {

    let data = body.into_inner();

    let module_content = module_contents::ActiveModel {

        module_id: Set(data.module_id),

        content_type: Set(data.content_type),

        content_category: Set(data.content_category),

        title: Set(data.title),

        content_url: Set(data.content_url),

        cloudinary_public_id: Set(data.cloudinary_public_id),

        position: Set(data.position),

        ..Default::default()
    };

    match module_content.insert(db.get_ref()).await {

        Ok(result) => {
            HttpResponse::Ok().json(result)
        }

        Err(err) => {
            HttpResponse::InternalServerError()
                .body(format!("Insert error: {}", err))
        }
    }
}

#[delete("/module-content/{module_content_id}")]
pub async fn delete_module_content(
    db:web::Data<DatabaseConnection>, 
    path:web::Path<i32>
)-> impl Responder {
    let module_content_id = path.into_inner();
    let existing = module_contents::Entity::find_by_id(module_content_id)
    .one(db.get_ref())
    .await;

    match existing {
        Ok(Some(module_content)) => {
            let active_model:module_contents::ActiveModel = module_content.into();
            match active_model.delete(db.get_ref()).await {
                Ok(_) => {
                    HttpResponse::Ok()
                    .body("Module content deleted!")
                }
                Err(err) => {
                    HttpResponse::InternalServerError()
                    .body(format!("Delete error: {}", err))
                }
            }
        }
        Ok(None) => {
            HttpResponse::NotFound()
            .body("Module content not found!")
        }
        Err(err) => {
            HttpResponse::InternalServerError()
            .body(format!("Delete error {}", err))
        }
    }
}
