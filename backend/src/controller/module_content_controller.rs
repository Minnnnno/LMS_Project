use crate::entity::{courses, module_contents, modules, users};
use crate::models::module_content::{CreateModuleContent, UpdateModuleContent};
use crate::services::auth_helpers::get_user_id;
use crate::services::course_service::has_role;
use crate::services::module_content_service::{can_manage_module_content, can_view_module_content};
use actix_session::Session;
use actix_web::{HttpResponse, Responder, delete, get, post, put, web};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

#[get("/module-content")]
pub async fn get_module_contents(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    let user_id = match get_user_id(&session) {
        Ok(user_id) => user_id,
        Err(response) => return response,
    };

    let result = if has_role(&session, "LMS Admin") {
        module_contents::Entity::find().all(db.get_ref()).await
    } else if has_role(&session, "Organisation Admin") || has_role(&session, "Instructor") {
        let user = match users::Entity::find_by_id(user_id).one(db.get_ref()).await {
            Ok(Some(user)) => user,
            Ok(None) => return HttpResponse::NotFound().body("User not found"),
            Err(err) => {
                return HttpResponse::InternalServerError()
                    .body(format!("Database error finding user: {}", err));
            }
        };

        let course_query = if has_role(&session, "Organisation Admin") {
            let org_id = match user.org_id {
                Some(org_id) => org_id,
                None => {
                    return HttpResponse::Forbidden()
                        .body("Organisation Admin is not assigned to an organisation");
                }
            };

            courses::Entity::find().filter(courses::Column::OrgId.eq(org_id))
        } else {
            courses::Entity::find().filter(courses::Column::InstructorId.eq(user_id))
        };

        let course_ids: Vec<i32> = match course_query.all(db.get_ref()).await {
            Ok(courses) => courses.into_iter().map(|course| course.course_id).collect(),
            Err(err) => {
                return HttpResponse::InternalServerError().body(format!(
                    "Database error finding organisation courses: {}",
                    err
                ));
            }
        };

        let module_ids: Vec<i32> = match modules::Entity::find()
            .filter(modules::Column::CourseId.is_in(course_ids))
            .all(db.get_ref())
            .await
        {
            Ok(modules) => modules.into_iter().map(|module| module.module_id).collect(),
            Err(err) => {
                return HttpResponse::InternalServerError().body(format!(
                    "Database error finding organisation modules: {}",
                    err
                ));
            }
        };

        module_contents::Entity::find()
            .filter(module_contents::Column::ModuleId.is_in(module_ids))
            .all(db.get_ref())
            .await
    } else {
        return HttpResponse::Forbidden().body("Admin role required to list module content");
    };

    match result {
        Ok(module_content) => {
            if module_content.is_empty() {
                HttpResponse::NotFound().body("No module content found")
            } else {
                HttpResponse::Ok().json(module_content)
            }
        }
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

#[get("/module-content/{module_id}")]
pub async fn get_module_content_by_id(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    let module_id = path.into_inner();

    match get_user_id(&session) {
        Ok(_) => {}
        Err(response) => return response,
    };

    match can_view_module_content(db.get_ref(), &session, module_id).await {
        Ok(true) => {}
        Ok(false) => {
            return HttpResponse::Forbidden()
                .body("You must be enrolled to view this module content");
        }
        Err(response) => return response,
    }

    let result = module_contents::Entity::find()
        .filter(module_contents::Column::ModuleId.eq(module_id))
        .all(db.get_ref())
        .await;
    match result {
        Ok(module_content) => {
            if module_content.is_empty() {
                HttpResponse::NotFound().body("No module content found")
            } else {
                HttpResponse::Ok().json(module_content)
            }
        }
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

#[get("/module-content/{module_id}/manage-access")]
pub async fn get_module_content_manage_access(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    let module_id = path.into_inner();

    match can_manage_module_content(db.get_ref(), &session, module_id).await {
        Ok(can_manage) => HttpResponse::Ok().json(serde_json::json!({
            "can_manage": can_manage
        })),
        Err(response) => response,
    }
}

#[put("/module-content/{module_content_id}")]
pub async fn update_module_content(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    body: web::Json<UpdateModuleContent>,
) -> impl Responder {
    let module_content_id = path.into_inner();
    let data = body.into_inner();
    let existing = module_contents::Entity::find_by_id(module_content_id)
        .one(db.get_ref())
        .await;

    match existing {
        Ok(Some(module_content)) => {
            let target_module_id = data.module_id.unwrap_or(module_content.module_id);

            match can_manage_module_content(db.get_ref(), &session, target_module_id).await {
                Ok(true) => {}
                Ok(false) => {
                    return HttpResponse::Forbidden().body(
                        "Organisation Admin can only update content under their organisation",
                    );
                }
                Err(response) => return response,
            }

            let mut active: module_contents::ActiveModel = module_content.into();

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
                Ok(_) => HttpResponse::Ok().body(format!(
                    "Module content with id {} updated!",
                    module_content_id
                )),
                Err(err) => {
                    HttpResponse::InternalServerError().body(format!("Update error: {}", err))
                }
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Module content not found"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    }
}

#[post("/module-content")]
pub async fn create_module_content(
    db: web::Data<DatabaseConnection>,
    session: Session,
    body: web::Json<CreateModuleContent>,
) -> impl Responder {
    let data = body.into_inner();

    match can_manage_module_content(db.get_ref(), &session, data.module_id).await {
        Ok(true) => {}
        Ok(false) => {
            return HttpResponse::Forbidden()
                .body("Organisation Admin can only create content under their organisation");
        }
        Err(response) => return response,
    }

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
        Ok(result) => HttpResponse::Ok().json(result),

        Err(err) => HttpResponse::InternalServerError().body(format!("Insert error: {}", err)),
    }
}

#[delete("/module-content/{module_content_id}")]
pub async fn delete_module_content(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    let module_content_id = path.into_inner();
    let existing = module_contents::Entity::find_by_id(module_content_id)
        .one(db.get_ref())
        .await;

    match existing {
        Ok(Some(module_content)) => {
            match can_manage_module_content(db.get_ref(), &session, module_content.module_id).await
            {
                Ok(true) => {}
                Ok(false) => {
                    return HttpResponse::Forbidden().body(
                        "Organisation Admin can only delete content under their organisation",
                    );
                }
                Err(response) => return response,
            }

            let active_model: module_contents::ActiveModel = module_content.into();
            match active_model.delete(db.get_ref()).await {
                Ok(_) => HttpResponse::Ok().body("Module content deleted!"),
                Err(err) => {
                    HttpResponse::InternalServerError().body(format!("Delete error: {}", err))
                }
            }
        }
        Ok(None) => HttpResponse::NotFound().body("Module content not found!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Delete error {}", err)),
    }
}
