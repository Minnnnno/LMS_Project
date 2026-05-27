use actix_session::Session;
use actix_web::{get, put, web, HttpResponse, Responder};

use sea_orm::{
    ActiveModelTrait, DatabaseConnection, EntityTrait, IntoActiveModel, Set,
};

use validator::Validate;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordVerifier, SaltString},
    Argon2, PasswordHasher,
};

use crate::entity::users;
use crate::models::student::{UpdateOwnProfileForm, ChangePasswordForm};



//helper function to get user session to distinguish between user
fn get_session_user_id(session: &Session) -> Result<i32, HttpResponse> {
    match session.get::<i32>("user_id") {
        Ok(Some(user_id)) => Ok(user_id),             // if user_id is found in session, return it
        Ok(None) => Err(HttpResponse::Unauthorized().body("User not logged in")),  // if user_id is not found in session, return unauthorized

        Err(_) => Err(HttpResponse::InternalServerError().body("Failed to retrieve session")),  // if there is an error retrieving session, return internal server error
    }
}



//profile view
#[get("/student/profile")]
pub async fn get_own_profile(
    db: web::Data<DatabaseConnection>,
    session: Session,
)   -> impl Responder {
    let user_id = match get_session_user_id(&session) {
        Ok(id) => id,
        Err(err) => return err,  // if there is an error getting user_id from session, return the error response
    } ;     //helper function already handles Ok(None) response and Err() response 


    match users::Entity::find_by_id(user_id).one(db.get_ref()).await {
        Ok(Some(user)) => HttpResponse::Ok().json(user),                                        // if user is found, return it as JSON
        Ok(None) => HttpResponse::NotFound().body("User not found in database"),               // if user is not found, return not found
        Err(_) => HttpResponse::InternalServerError().body("Failed to retrieve user"),          // if there is an error retrieving user, return internal server error
    }

}




//update profile view
#[put("/student/profile")]
pub async fn update_own_profile(
    db:web::Data<DatabaseConnection>,
    session: Session,
    body: web::Json<UpdateOwnProfileForm>,
) -> impl Responder {
    let user_id = match get_session_user_id(&session) {
        Ok(id) => id,
        Err(err) => return err,  // if there is an error getting user_id from session, return the error response
    };      //helper function already handles Ok(None) response and Err() response



    let user = match users::Entity::find_by_id(user_id).one(db.get_ref()).await {
        Ok(Some(user)) => user,                                        // if user is found, return it as JSON
        Ok(None) => return HttpResponse::NotFound().body("User not found in database"),               // if user is not found, return not found
        Err(_) => return HttpResponse::InternalServerError().body("Failed to retrieve user"),          // if there is an error retrieving user, return internal server error
    };


    let mut update_user = user.into_active_model();  // convert user to active model to update it
    update_user.first_name = Set(body.first_name.trim().to_string());  // update first name
    update_user.last_name = Set(body.last_name.trim().to_string());    // update last name
    update_user.email = Set(body.email.trim().to_string());             // update email

    match update_user.update(db.get_ref()).await {
        Ok(updated_user) => HttpResponse::Ok().json(updated_user),  // if update is successful, return updated user as JSON
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Update profile error: {}", err)),  // if there is an error updating user, return internal server error
    }
}


//Password changing 
#[put("/student/password")]
pub async fn change_password(
    db:web::Data<DatabaseConnection>,
    session: Session,
    body: web::Json<ChangePasswordForm>,
) -> impl Responder {
    let user_id = match get_session_user_id(&session) {
        Ok(id) => id,
        Err(err) => return err,  // if there is an error getting user_id from session, return the error response
    } ;     //helper function already handles Ok(None) response and Err() response

    //check if password form follows validation rules from model(ChangePasswordForm struct)
    if let Err(err) = body.validate() {
        return HttpResponse::BadRequest().body(format!("Validation error: {}", err));  // if there is a validation error, return bad request with error message
    }

    //check if new password and confirm password match
    if body.new_password != body.confirm_password {
        return HttpResponse::BadRequest().body("New password and confirm password do not match!");  // if new password and confirm password do not match, return bad request
    }

    //find user in database
    let user = match users::Entity::find_by_id(user_id)
        .one(db.get_ref())
        .await
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            return HttpResponse::NotFound()
                .body("User not found");
        }
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error: {}", err));
        }
    };

    //parse existing password hash from database for argon2 password verification
    let parsed_hash = match PasswordHash::new(&user.password_hash) {
        Ok(hash) => hash,     // if password hash is successfully parsed, return it
        Err(err) => {           // if there is an error parsing password hash, return internal server error with error message
            println!("Password hash parse error: {:?}", err);
            return HttpResponse::InternalServerError()
                .body("Failed to verify password");
        }
    };


    //verify current password with argon2
    if Argon2::default()
        .verify_password(body.current_password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return HttpResponse::Unauthorized().body("Current password is incorrect!");  // if current password is incorrect, return unauthorized
    }


    let new_password = body.new_password.clone(); 


    //hash new password with argon2
    let password_hash_result = actix_web::rt::task::spawn_blocking(move || {
        let salt = SaltString::generate(&mut OsRng);  // generate random salt for password hashing
        
        Argon2::default()
            .hash_password(new_password.as_bytes(), &salt)
            .map(|hash| hash.to_string())  // if password is successfully hashed, return the hash as string
    })
    .await;

    let new_password_hash = match password_hash_result {
        Ok(Ok(hash)) => hash,  // if password hashing is successful, return the hash
        Ok(Err(err)) => {      // if there is an error hashing password, return internal server error with error message
            println!("Password hashing error: {:?}", err);
            return HttpResponse::InternalServerError()
                .body("Failed to hash new password");
        }
        Err(err) => {           // if there is an error in the blocking task, return internal server error with error message
            println!("Blocking task error: {:?}", err);
            return HttpResponse::InternalServerError()
                .body("Failed to hash new password");
        }
    };

    //update user's password hash in database
    let mut update_user = user.into_active_model();  // convert user to active model to update it
    update_user.password_hash = Set(new_password_hash);  // set new password hash

    match update_user.update(db.get_ref()).await {
        Ok(_) => HttpResponse::Ok().body("Password changed successfully!"),  // if update is successful, return success message
        Err(err) => HttpResponse::InternalServerError().body(format!("Failed to update password: {}", err)),  // if there is an error updating user, return internal server error with error message
    }



}












