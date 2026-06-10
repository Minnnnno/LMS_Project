use actix_web::{HttpResponse, Responder, get, web, post, put, delete};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, Set, ActiveModelTrait, QueryOrder};
use crate::entity::modules; 
use crate::models::modules::{
    CreateModules,
    UpdateModules,
};
use crate::services::module_service::reorder_modules_for_course;

#[get("/modules")]
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

#[get("/modules/{course_id}")]
pub async fn get_modules_by_course_id(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>
) -> impl Responder {
    let course_id = path.into_inner(); 
    let result = modules::Entity::find()
    .filter(modules::Column::CourseId.eq(course_id))
    .order_by_asc(modules::Column::Position)
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
            let current_course_id = module.course_id;
            let current_position = module.position;
            let requested_course_id = data.course_id.unwrap_or(current_course_id);
            let requested_position = data.position.unwrap_or(current_position);

            if requested_course_id != current_course_id {
                return HttpResponse::BadRequest()
                    .body("Moving modules between courses is not supported here");
            }

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
                Ok(_) => {
                    if let Err(response) = reorder_modules_for_course(
                        db.get_ref(),
                        current_course_id,
                        Some(module_id),
                        requested_position,
                    ).await {
                        return response;
                    }

                    HttpResponse::Ok()
                        .body(format!("module with id {} updated!", module_id))
                }
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

    if data.position < 1 {
        return HttpResponse::BadRequest().body("Module position must be 1 or higher");
    }

    let requested_position = data.position;
    let course_id = data.course_id;

    let module = modules::ActiveModel {
        course_id: Set(data.course_id),
        title: Set(data.title),
        position: Set(requested_position),
        ..Default::default()
    };

    match module.insert(db.get_ref()).await {

        Ok(result) => {
            if let Err(response) = reorder_modules_for_course(
                db.get_ref(),
                course_id,
                Some(result.module_id),
                requested_position,
            ).await {
                return response;
            }

            HttpResponse::Ok()
                .body("New module created successfully!")
        }

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
            let course_id = module.course_id;
            let active_model:modules::ActiveModel = module.into();
            match active_model.delete(db.get_ref()).await {
                Ok(_) => {
                    if let Err(response) = reorder_modules_for_course(
                        db.get_ref(),
                        course_id,
                        None,
                        1,
                    ).await {
                        return response;
                    }

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
