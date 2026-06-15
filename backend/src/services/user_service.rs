use actix_session::Session;
use actix_web::{http::header, HttpResponse};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, Set,
};
use std::env;

use crate::entity::{roles, user_roles, users};

pub fn redirect_home() -> HttpResponse {
    HttpResponse::Found()
        .insert_header((header::LOCATION, "/"))
        .finish()
}

pub fn is_logged_in(session: &Session) -> bool {
    session.get::<i32>("user_id").ok().flatten().is_some()
}

pub fn role_name_to_string(role_name: roles::RoleName) -> String {
    match role_name {
        roles::RoleName::LmsAdmin => "LMS Admin",
        roles::RoleName::OrganisationAdmin => "Organisation Admin",
        roles::RoleName::Instructor => "Instructor",
        roles::RoleName::Student => "Student",
    }
    .to_string()
}

pub async fn load_user_roles(
    db: &DatabaseConnection,
    user_id: i32,
) -> Result<(Vec<i32>, Vec<String>), DbErr> {
    let user_role_rows = user_roles::Entity::find()
        .filter(user_roles::Column::UserId.eq(user_id))
        .all(db)
        .await?;

    let role_ids: Vec<i32> = user_role_rows
        .iter()
        .map(|user_role| user_role.role_id)
        .collect();

    if role_ids.is_empty() {
        return Ok((role_ids, Vec::new()));
    }

    let role_names = roles::Entity::find()
        .filter(roles::Column::RoleId.is_in(role_ids.clone()))
        .all(db)
        .await?
        .into_iter()
        .map(|role| role_name_to_string(role.role_name))
        .collect::<Vec<String>>();

    Ok((role_ids, role_names))
}

pub fn store_roles_in_session(session: &Session, role_ids: Vec<i32>, role_names: Vec<String>) {
    if let Err(err) = session.insert("role_ids", role_ids) {
        println!("Session insert error: {:?}", err);
    }
    if let Err(err) = session.insert("role_names", role_names) {
        println!("Session insert error: {:?}", err);
    }
}

pub async fn assign_role_to_user<C>(
    db: &C,
    user_id: i32,
    role_name: roles::RoleName,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let role = roles::Entity::find()
        .filter(roles::Column::RoleName.eq(role_name))
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("Role not found in database.".to_string()))?;

    let new_user_role = user_roles::ActiveModel {
        user_id: Set(user_id),
        role_id: Set(role.role_id),
    };

    new_user_role.insert(db).await?;

    Ok(())
}

pub fn google_client_id() -> Result<String, String> {
    env::var("GOOGLE_CLIENT_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Google login is not configured. Missing GOOGLE_CLIENT_ID.".to_string())
}

pub fn google_client_secret() -> Result<String, String> {
    env::var("GOOGLE_CLIENT_SECRET")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Google login is not configured. Missing GOOGLE_CLIENT_SECRET.".to_string())
}

pub fn google_redirect_uri() -> String {
    env::var("GOOGLE_REDIRECT_URI")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "http://127.0.0.1:8080/auth/google/callback".to_string())
}

pub async fn hash_password(password: String) -> Result<String, String> {
    let password_hash_result = actix_web::rt::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);

        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
    })
    .await;

    match password_hash_result {
        Ok(Ok(hash)) => Ok(hash),
        Ok(Err(err)) => {
            println!("Password hashing error: {:?}", err);
            Err("Failed to process password.".to_string())
        }
        Err(err) => {
            println!("Hashing task error: {:?}", err);
            Err("Failed to process password.".to_string())
        }
    }
}

pub async fn sign_user_into_session(
    db: &DatabaseConnection,
    session: &Session,
    user: &users::Model,
) -> Result<(), String> {
    let (role_ids, role_names) = load_user_roles(db, user.user_id).await.map_err(|err| {
        println!("User role lookup error: {:?}", err);
        "Unable to process login at this time.".to_string()
    })?;

    session.renew();

    if let Err(err) = session.insert("user_id", user.user_id) {
        println!("Session insert error: {:?}", err);
    }
    if let Err(err) = session.insert("user_email", user.email.clone()) {
        println!("Session insert error: {:?}", err);
    }
    if let Err(err) = session.insert("email_verified", user.email_verified) {
        println!("Session insert error: {:?}", err);
    }
    if let Err(err) = session.insert("must_change_password", user.must_change_password) {
        println!("Session insert error: {:?}", err);
    }
    store_roles_in_session(session, role_ids, role_names);

    Ok(())
}
