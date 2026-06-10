use actix_web::HttpResponse;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};

use crate::entity::modules;

pub async fn reorder_modules_for_course(
    db: &DatabaseConnection,
    course_id: i32,
    moving_module_id: Option<i32>,
    requested_position: i32,
) -> Result<(), HttpResponse> {
    if requested_position < 1 {
        return Err(HttpResponse::BadRequest().body("Module position must be 1 or higher"));
    }

    let siblings = modules::Entity::find()
        .filter(modules::Column::CourseId.eq(course_id))
        .order_by_asc(modules::Column::Position)
        .all(db)
        .await
        .map_err(|err| {
            HttpResponse::InternalServerError()
                .body(format!("Database error finding modules: {}", err))
        })?;

    let mut reordered: Vec<modules::Model> = siblings
        .into_iter()
        .filter(|module| Some(module.module_id) != moving_module_id)
        .collect();

    let insert_index = (requested_position as usize).saturating_sub(1).min(reordered.len());

    if let Some(module_id) = moving_module_id {
        let moving_module = modules::Entity::find_by_id(module_id)
            .one(db)
            .await
            .map_err(|err| {
                HttpResponse::InternalServerError()
                    .body(format!("Database error finding module: {}", err))
            })?
            .ok_or_else(|| HttpResponse::NotFound().body("Module not found"))?;

        reordered.insert(insert_index, moving_module);
    }

    for (index, module) in reordered.into_iter().enumerate() {
        let new_position = index as i32 + 1;

        if module.position != new_position {
            let mut active: modules::ActiveModel = module.into();
            active.position = Set(new_position);
            active.update(db).await.map_err(|err| {
                HttpResponse::InternalServerError()
                    .body(format!("Module reorder error: {}", err))
            })?;
        }
    }

    Ok(())
}
