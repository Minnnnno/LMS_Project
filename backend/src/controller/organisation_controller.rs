use actix_session::Session;
use actix_web::{delete, get, post, web, HttpResponse, Responder};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    TransactionTrait,
};

use crate::entity::{organisations, roles, user_roles, users};
use crate::models::organisation::{CreateOrganisationForm, MassEnrollForm, OrgMemberDto};
use crate::ssr::pages::render_page;

// ── Session helpers ────────────────────────────────────────────────────────────

fn get_session_user_id(session: &Session) -> Result<i32, HttpResponse> {
    match session.get::<i32>("user_id") {
        Ok(Some(id)) => Ok(id),
        Ok(None) => Err(HttpResponse::Unauthorized().body("User not logged in")),
        Err(_) => Err(HttpResponse::InternalServerError().body("Failed to retrieve session")),
    }
}

fn require_org_admin(session: &Session) -> Result<(), HttpResponse> {
    let role_names: Vec<String> = session
        .get::<Vec<String>>("role_names")
        .ok()
        .flatten()
        .unwrap_or_default();

    if role_names
        .iter()
        .any(|r| r == "Organisation Admin" || r == "LMS Admin")
    {
        Ok(())
    } else {
        Err(HttpResponse::Forbidden().body("Organisation Admin or LMS Admin role required"))
    }
}

// ── Page route ─────────────────────────────────────────────────────────────────

#[get("/organisation")]
pub async fn organisation_page(session: Session) -> impl Responder {
    render_page("organisation.html", &session)
}

// ── CRUD: organisations ────────────────────────────────────────────────────────

/// GET /api/organisations  –  list all organisations
#[get("/organisations")]
pub async fn list_organisations(db: web::Data<DatabaseConnection>) -> impl Responder {
    match organisations::Entity::find().all(db.get_ref()).await {
        Ok(orgs) => HttpResponse::Ok().json(orgs),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

/// GET /api/organisations/{org_id}  –  single organisation
#[get("/organisations/{org_id}")]
pub async fn get_organisation(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> impl Responder {
    let org_id = path.into_inner();
    match organisations::Entity::find_by_id(org_id)
        .one(db.get_ref())
        .await
    {
        Ok(Some(org)) => HttpResponse::Ok().json(org),
        Ok(None) => HttpResponse::NotFound().body("Organisation not found"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

/// POST /api/organisations  –  create organisation (admin only)
#[post("/organisations")]
pub async fn create_organisation(
    db: web::Data<DatabaseConnection>,
    session: Session,
    body: web::Json<CreateOrganisationForm>,
) -> impl Responder {
    if let Err(e) = require_org_admin(&session) {
        return e;
    }

    let new_org = organisations::ActiveModel {
        org_name: Set(body.org_name.trim().to_string()),
        ..Default::default()
    };

    match new_org.insert(db.get_ref()).await {
        Ok(org) => HttpResponse::Ok().json(org),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Failed to create organisation: {}", err)),
    }
}

/// DELETE /api/organisations/{org_id}  –  delete organisation (admin only)
#[delete("/organisations/{org_id}")]
pub async fn delete_organisation(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
) -> impl Responder {
    if let Err(e) = require_org_admin(&session) {
        return e;
    }

    let org_id = path.into_inner();
    match organisations::Entity::delete_by_id(org_id)
        .exec(db.get_ref())
        .await
    {
        Ok(res) if res.rows_affected > 0 => {
            HttpResponse::Ok().body("Organisation deleted")
        }
        Ok(_) => HttpResponse::NotFound().body("Organisation not found"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Failed to delete organisation: {}", err)),
    }
}

// ── Members ────────────────────────────────────────────────────────────────────

/// GET /api/organisations/{org_id}/members  –  list all members with their roles
#[get("/organisations/{org_id}/members")]
pub async fn list_org_members(
    db: web::Data<DatabaseConnection>,
    path: web::Path<i32>,
) -> impl Responder {
    let org_id = path.into_inner();

    let members = match users::Entity::find()
        .filter(users::Column::OrgId.eq(org_id))
        .all(db.get_ref())
        .await
    {
        Ok(m) => m,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error fetching members: {}", err))
        }
    };

    // fetch all roles (small table – load once)
    let all_roles = match roles::Entity::find().all(db.get_ref()).await {
        Ok(r) => r,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error fetching roles: {}", err))
        }
    };

    let mut dtos: Vec<OrgMemberDto> = Vec::new();

    for user in members {
        let user_role_rows = match user_roles::Entity::find()
            .filter(user_roles::Column::UserId.eq(user.user_id))
            .all(db.get_ref())
            .await
        {
            Ok(rows) => rows,
            Err(_) => vec![],
        };

        let role_names: Vec<String> = user_role_rows
            .iter()
            .filter_map(|ur| {
                all_roles
                    .iter()
                    .find(|r| r.role_id == ur.role_id)
                    .map(|r| format!("{:?}", r.role_name)
                        .replace("LmsAdmin", "LMS Admin")
                        .replace("OrganisationAdmin", "Organisation Admin")
                        .replace("Instructor", "Instructor")
                        .replace("Student", "Student"))
            })
            .collect();

        // Use the sea_orm string value instead of Debug
        let role_names: Vec<String> = user_role_rows
            .iter()
            .filter_map(|ur| {
                all_roles.iter().find(|r| r.role_id == ur.role_id).map(|r| {
                    match r.role_name {
                        roles::RoleName::LmsAdmin => "LMS Admin".to_string(),
                        roles::RoleName::OrganisationAdmin => "Organisation Admin".to_string(),
                        roles::RoleName::Instructor => "Instructor".to_string(),
                        roles::RoleName::Student => "Student".to_string(),
                    }
                })
            })
            .collect();

        dtos.push(OrgMemberDto {
            user_id: user.user_id,
            first_name: user.first_name,
            last_name: user.last_name,
            email: user.email,
            roles: role_names,
        });
    }

    HttpResponse::Ok().json(dtos)
}

// ── Mass enrollment ────────────────────────────────────────────────────────────

/// POST /api/organisations/{org_id}/enroll
///
/// Body: { "user_ids": [1, 2, 3], "role": "Student" | "Instructor" }
///
/// For each user_id:
///   1. Sets users.org_id = org_id
///   2. Assigns the requested role (idempotent – skips if already assigned)
#[post("/organisations/{org_id}/enroll")]
pub async fn mass_enroll(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<i32>,
    body: web::Json<MassEnrollForm>,
) -> impl Responder {
    if let Err(e) = require_org_admin(&session) {
        return e;
    }

    let org_id = path.into_inner();

    // Resolve the target role
    let target_role_name = match body.role.as_str() {
        "Instructor" => roles::RoleName::Instructor,
        "Student" => roles::RoleName::Student,
        other => {
            return HttpResponse::BadRequest()
                .body(format!("Invalid role '{}'. Use 'Instructor' or 'Student'.", other))
        }
    };

    let role_row = match roles::Entity::find()
        .filter(roles::Column::RoleName.eq(target_role_name))
        .one(db.get_ref())
        .await
    {
        Ok(Some(r)) => r,
        Ok(None) => return HttpResponse::InternalServerError().body("Role not found in database"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error looking up role: {}", err))
        }
    };

    let txn = match db.get_ref().begin().await {
        Ok(t) => t,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Failed to start transaction: {}", err))
        }
    };

    let mut enrolled: Vec<i32> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    for &uid in &body.user_ids {
        // 1. Fetch user
        let user = match users::Entity::find_by_id(uid).one(&txn).await {
            Ok(Some(u)) => u,
            Ok(None) => {
                errors.push(format!("User {} not found", uid));
                continue;
            }
            Err(err) => {
                errors.push(format!("DB error for user {}: {}", uid, err));
                continue;
            }
        };

        // 2. Set org_id on the user
        let mut active_user = sea_orm::IntoActiveModel::into_active_model(user);
        active_user.org_id = Set(Some(org_id));
        if let Err(err) = active_user.update(&txn).await {
            errors.push(format!("Failed to set org for user {}: {}", uid, err));
            continue;
        }

        // 3. Assign role (idempotent)
        let already_has_role = match user_roles::Entity::find()
            .filter(user_roles::Column::UserId.eq(uid))
            .filter(user_roles::Column::RoleId.eq(role_row.role_id))
            .one(&txn)
            .await
        {
            Ok(Some(_)) => true,
            Ok(None) => false,
            Err(err) => {
                errors.push(format!("Role check error for user {}: {}", uid, err));
                continue;
            }
        };

        if !already_has_role {
            let new_ur = user_roles::ActiveModel {
                user_id: Set(uid),
                role_id: Set(role_row.role_id),
            };
            if let Err(err) = new_ur.insert(&txn).await {
                errors.push(format!("Failed to assign role for user {}: {}", uid, err));
                continue;
            }
        }

        enrolled.push(uid);
    }

    if let Err(err) = txn.commit().await {
        return HttpResponse::InternalServerError()
            .body(format!("Failed to commit transaction: {}", err));
    }

    HttpResponse::Ok().json(serde_json::json!({
        "enrolled": enrolled,
        "errors": errors,
        "message": format!("{} user(s) enrolled successfully", enrolled.len())
    }))
}

/// DELETE /api/organisations/{org_id}/members/{user_id}
/// Removes a user from the organisation (sets org_id to NULL)
#[delete("/organisations/{org_id}/members/{user_id}")]
pub async fn remove_org_member(
    db: web::Data<DatabaseConnection>,
    session: Session,
    path: web::Path<(i32, i32)>,
) -> impl Responder {
    if let Err(e) = require_org_admin(&session) {
        return e;
    }

    let (_org_id, user_id) = path.into_inner();

    let user = match users::Entity::find_by_id(user_id).one(db.get_ref()).await {
        Ok(Some(u)) => u,
        Ok(None) => return HttpResponse::NotFound().body("User not found"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error: {}", err))
        }
    };

    let mut active_user = sea_orm::IntoActiveModel::into_active_model(user);
    active_user.org_id = Set(None);

    match active_user.update(db.get_ref()).await {
        Ok(_) => HttpResponse::Ok().body("Member removed from organisation"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Failed to remove member: {}", err)),
    }
}

/// GET /api/users/all  –  all users in the system (for CSV/Excel file matching)
#[get("/users/all")]
pub async fn list_all_users(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    if let Err(e) = require_org_admin(&session) {
        return e;
    }

    match users::Entity::find().all(db.get_ref()).await {
        Ok(users) => {
            let safe: Vec<serde_json::Value> = users
                .iter()
                .map(|u| {
                    serde_json::json!({
                        "user_id": u.user_id,
                        "first_name": u.first_name,
                        "last_name": u.last_name,
                        "email": u.email,
                    })
                })
                .collect();
            HttpResponse::Ok().json(safe)
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

/// GET /api/users/unassigned  –  users not yet in any organisation (for the picker)
#[get("/users/unassigned")]
pub async fn list_unassigned_users(
    db: web::Data<DatabaseConnection>,
    session: Session,
) -> impl Responder {
    if let Err(e) = require_org_admin(&session) {
        return e;
    }

    match users::Entity::find()
        .filter(users::Column::OrgId.is_null())
        .all(db.get_ref())
        .await
    {
        Ok(users) => {
            let safe: Vec<serde_json::Value> = users
                .iter()
                .map(|u| {
                    serde_json::json!({
                        "user_id": u.user_id,
                        "first_name": u.first_name,
                        "last_name": u.last_name,
                        "email": u.email,
                    })
                })
                .collect();
            HttpResponse::Ok().json(safe)
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}
