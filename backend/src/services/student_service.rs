use actix_session::Session;
use actix_web::HttpResponse;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, IntoActiveModel, Set};
use validator::Validate;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordVerifier, SaltString},
    Argon2, PasswordHasher,
};

use crate::entity::users;
use crate::models::student::{ChangePasswordForm, UpdateOwnProfileForm};
use crate::services::auth_helpers::get_user_id;

pub async fn get_own_profile(db: &DatabaseConnection, session: &Session) -> HttpResponse {
    let user_id = match get_user_id(session) {
        Ok(id) => id,
        Err(response) => return response,
    };

    match users::Entity::find_by_id(user_id).one(db).await {
        Ok(Some(user)) => HttpResponse::Ok().json(user),
        Ok(None) => HttpResponse::NotFound().body("User not found in database"),
        Err(_) => HttpResponse::InternalServerError().body("Failed to retrieve user"),
    }
}

pub async fn update_own_profile(
    db: &DatabaseConnection,
    session: &Session,
    body: UpdateOwnProfileForm,
) -> HttpResponse {
    let user_id = match get_user_id(session) {
        Ok(id) => id,
        Err(response) => return response,
    };

    let user = match users::Entity::find_by_id(user_id).one(db).await {
        Ok(Some(user)) => user,
        Ok(None) => return HttpResponse::NotFound().body("User not found in database"),
        Err(_) => return HttpResponse::InternalServerError().body("Failed to retrieve user"),
    };

    let mut update_user = user.into_active_model();
    update_user.first_name = Set(body.first_name.trim().to_string());
    update_user.last_name = Set(body.last_name.trim().to_string());
    update_user.email = Set(body.email.trim().to_string());

    match update_user.update(db).await {
        Ok(updated_user) => HttpResponse::Ok().json(updated_user),
        Err(err) => HttpResponse::InternalServerError().body(format!("Update profile error: {}", err)),
    }
}

pub async fn change_password(
    db: &DatabaseConnection,
    session: &Session,
    body: ChangePasswordForm,
) -> HttpResponse {
    let user_id = match get_user_id(session) {
        Ok(id) => id,
        Err(response) => return response,
    };

    if let Err(err) = body.validate() {
        return HttpResponse::BadRequest().body(format!("Validation error: {}", err));
    }

    if body.new_password != body.confirm_password {
        return HttpResponse::BadRequest().body("New password and confirm password do not match!");
    }

    let user = match users::Entity::find_by_id(user_id).one(db).await {
        Ok(Some(user)) => user,
        Ok(None) => return HttpResponse::NotFound().body("User not found"),
        Err(err) => return HttpResponse::InternalServerError().body(format!("Database error: {}", err)),
    };

    let password_hash = match &user.password_hash {
        Some(password_hash) => password_hash,
        None => {
            return HttpResponse::BadRequest()
                .body("This account does not have a password. Please sign in with Google.");
        }
    };

    let parsed_hash = match PasswordHash::new(password_hash) {
        Ok(hash) => hash,
        Err(err) => {
            println!("Password hash parse error: {:?}", err);
            return HttpResponse::InternalServerError().body("Failed to verify password");
        }
    };

    if Argon2::default()
        .verify_password(body.current_password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return HttpResponse::Unauthorized().body("Current password is incorrect!");
    }

    let new_password = body.new_password.clone();
    let password_hash_result = actix_web::rt::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(new_password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
    })
    .await;

    let new_password_hash = match password_hash_result {
        Ok(Ok(hash)) => hash,
        Ok(Err(err)) => {
            println!("Password hashing error: {:?}", err);
            return HttpResponse::InternalServerError().body("Failed to hash new password");
        }
        Err(err) => {
            println!("Blocking task error: {:?}", err);
            return HttpResponse::InternalServerError().body("Failed to hash new password");
        }
    };

    let mut update_user = user.into_active_model();
    update_user.password_hash = Set(Some(new_password_hash));
    update_user.auth_provider = Set("password".to_string());

    match update_user.update(db).await {
        Ok(_) => HttpResponse::Ok().body("Password changed successfully!"),
        Err(err) => HttpResponse::InternalServerError().body(format!("Failed to update password: {}", err)),
    }
}
