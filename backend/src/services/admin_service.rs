use actix_web::HttpResponse;
use sea_orm::{
    ActiveModelTrait, DatabaseConnection, EntityTrait, IntoActiveModel, Set,
};
use validator::Validate;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};

use crate::entity::{
    organisations,
    users,
    courses,
    enrollments,
    user_roles,
};
use crate::entity::courses::CourseStatus;

use crate::models::admin::{
    CreateOrganisationForm,
    UpdateOrganisationForm,
    CreateAdminUserForm,
    UpdateAdminUserForm,
    CreateAdminCourseForm,
    UpdateAdminCourseForm,
    AdminEnrollmentForm,
};
// Password Hash Helper
fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);

    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|err| err.to_string())
}


fn parse_course_status(status: Option<String>) -> Result<CourseStatus, HttpResponse> {
    match status
        .unwrap_or_else(|| "draft".to_string())
        .to_lowercase()
        .as_str()
    {
        "draft" => Ok(CourseStatus::Draft),
        "published" => Ok(CourseStatus::Published),
        "archived" => Ok(CourseStatus::Archived),
        _ => Err(
            HttpResponse::BadRequest()
                .body("Invalid course status. Use draft, published, or archived")
        ),
    }
}


// Organisation CRUD
pub async fn get_all_organisations(
    db: &DatabaseConnection,
) -> HttpResponse {
    match organisations::Entity::find().all(db).await {
        Ok(orgs) => HttpResponse::Ok().json(orgs),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

pub async fn create_organisation_service(
    db: &DatabaseConnection,
    body: CreateOrganisationForm,
) -> HttpResponse {
    if let Err(errors) = body.validate() {
        return HttpResponse::BadRequest()
            .body(format!("Validation error: {}", errors));
    }

    let new_org = organisations::ActiveModel {
        org_name: Set(body.org_name.trim().to_string()),
        ..Default::default()
    };

    match new_org.insert(db).await {
        Ok(org) => HttpResponse::Ok().json(org),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Create organisation error: {}", err)),
    }
}

pub async fn update_organisation_service(
    db: &DatabaseConnection,
    org_id: i32,
    body: UpdateOrganisationForm,
) -> HttpResponse {
    if let Err(errors) = body.validate() {
        return HttpResponse::BadRequest()
            .body(format!("Validation error: {}", errors));
    }

    let org = match organisations::Entity::find_by_id(org_id).one(db).await {
        Ok(Some(org)) => org,
        Ok(None) => return HttpResponse::NotFound().body("Organisation not found"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error: {}", err));
        }
    };

    let mut active_org = org.into_active_model();
    active_org.org_name = Set(body.org_name.trim().to_string());

    match active_org.update(db).await {
        Ok(updated_org) => HttpResponse::Ok().json(updated_org),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Update organisation error: {}", err)),
    }
}

pub async fn delete_organisation_service(
    db: &DatabaseConnection,
    org_id: i32,
) -> HttpResponse {
    match organisations::Entity::delete_by_id(org_id).exec(db).await {
        Ok(result) => {
            if result.rows_affected == 0 {
                HttpResponse::NotFound().body("Organisation not found")
            } else {
                HttpResponse::Ok().body("Organisation deleted successfully")
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Delete organisation error: {}", err)),
    }
}

// User CRUD
pub async fn get_all_users(
    db: &DatabaseConnection,
) -> HttpResponse {
    match users::Entity::find().all(db).await {
        Ok(users_list) => HttpResponse::Ok().json(users_list),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

pub async fn get_user_by_id_service(
    db: &DatabaseConnection,
    user_id: i32,
) -> HttpResponse {
    match users::Entity::find_by_id(user_id).one(db).await {
        Ok(Some(user)) => HttpResponse::Ok().json(user),
        Ok(None) => HttpResponse::NotFound().body("User not found"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

pub async fn create_user_service(
    db: &DatabaseConnection,
    body: CreateAdminUserForm,
) -> HttpResponse {
    if let Err(errors) = body.validate() {
        return HttpResponse::BadRequest()
            .body(format!("Validation error: {}", errors));
    }

    let password_hash = match hash_password(&body.password) {
        Ok(hash) => hash,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Password hash error: {}", err));
        }
    };

    let role_id = body.role_id;

    let new_user = users::ActiveModel {
        first_name: Set(body.first_name.trim().to_string()),
        last_name: Set(body.last_name.trim().to_string()),
        email: Set(body.email.trim().to_lowercase()),
        password_hash: Set(Some(password_hash)),
        org_id: Set(body.org_id),
        email_verified: Set(false),
        ..Default::default()
    };

    let inserted_user = match new_user.insert(db).await {
        Ok(user) => user,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Create user error: {}", err));
        }
    };

    if let Some(role_id) = role_id {
        let new_user_role = user_roles::ActiveModel {
            user_id: Set(inserted_user.user_id),
            role_id: Set(role_id),
        };

        if let Err(err) = new_user_role.insert(db).await {
            return HttpResponse::InternalServerError()
                .body(format!("User created, but failed to assign role: {}", err));
        }
    }

    HttpResponse::Ok().json(inserted_user)
}


pub async fn update_user_service(
    db: &DatabaseConnection,
    user_id: i32,
    body: UpdateAdminUserForm,
) -> HttpResponse {
    if let Err(errors) = body.validate() {
        return HttpResponse::BadRequest()
            .body(format!("Validation error: {}", errors));
    }

    let user = match users::Entity::find_by_id(user_id).one(db).await {
        Ok(Some(user)) => user,
        Ok(None) => return HttpResponse::NotFound().body("User not found"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error: {}", err));
        }
    };

    let mut active_user = user.into_active_model();

    active_user.first_name = Set(body.first_name.trim().to_string());
    active_user.last_name = Set(body.last_name.trim().to_string());
    active_user.email = Set(body.email.trim().to_lowercase());
    active_user.org_id = Set(body.org_id);

    match active_user.update(db).await {
        Ok(updated_user) => HttpResponse::Ok().json(updated_user),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Update user error: {}", err)),
    }
} 


pub async fn delete_user_service(
    db: &DatabaseConnection,
    user_id: i32,
) -> HttpResponse {
    match users::Entity::delete_by_id(user_id).exec(db).await {
        Ok(result) => {
            if result.rows_affected == 0 {
                HttpResponse::NotFound().body("User not found")
            } else {
                HttpResponse::Ok().body("User deleted successfully")
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Delete user error: {}", err)),
    }
}

// Course CRUD
pub async fn get_all_courses(
    db: &DatabaseConnection,
) -> HttpResponse {
    match courses::Entity::find().all(db).await {
        Ok(course_list) => HttpResponse::Ok().json(course_list),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

pub async fn get_course_by_id_service(
    db: &DatabaseConnection,
    course_id: i32,
) -> HttpResponse {
    match courses::Entity::find_by_id(course_id).one(db).await {
        Ok(Some(course)) => HttpResponse::Ok().json(course),
        Ok(None) => HttpResponse::NotFound().body("Course not found"),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

pub async fn create_course_service(
    db: &DatabaseConnection,
    body: CreateAdminCourseForm,
) -> HttpResponse {
    if let Err(errors) = body.validate() {
        return HttpResponse::BadRequest()
            .body(format!("Validation error: {}", errors));
    }

    let status = match parse_course_status(body.status) {
        Ok(status) => status,
        Err(response) => return response,
    };

    let new_course = courses::ActiveModel {
        name: Set(Some(body.name.trim().to_string())),
        org_id: Set(body.org_id),
        instructor_id: Set(body.instructor_id),
        status: Set(status),
        price_cents: Set(Some(body.price_cents.unwrap_or(0))),
        currency: Set(Some(body.currency.unwrap_or_else(|| "sgd".to_string()))),
        is_paid: Set(Some(body.is_paid.unwrap_or(false))),
        description: Set(body.description),
        background_image_url: Set(body.background_image_url),
        ..Default::default()
    };

    match new_course.insert(db).await {
        Ok(course) => HttpResponse::Ok().json(course),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Create course error: {}", err)),
    }
}
pub async fn update_course_service(
    db: &DatabaseConnection,
    course_id: i32,
    body: UpdateAdminCourseForm,
) -> HttpResponse {
    if let Err(errors) = body.validate() {
        return HttpResponse::BadRequest()
            .body(format!("Validation error: {}", errors));
    }

    let course = match courses::Entity::find_by_id(course_id).one(db).await {
        Ok(Some(course)) => course,
        Ok(None) => return HttpResponse::NotFound().body("Course not found"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error: {}", err));
        }
    };

    let mut active_course = course.into_active_model();

    active_course.name = Set(Some(body.name.trim().to_string()));
    active_course.org_id = Set(body.org_id);
    active_course.instructor_id = Set(body.instructor_id);

    if body.status.is_some() {
        let status = match parse_course_status(body.status) {
            Ok(status) => status,
            Err(response) => return response,
        };

        active_course.status = Set(status);
    }

    if let Some(price_cents) = body.price_cents {
        active_course.price_cents = Set(Some(price_cents));
    }

    if let Some(currency) = body.currency {
        active_course.currency = Set(Some(currency));
    }

    if let Some(is_paid) = body.is_paid {
        active_course.is_paid = Set(Some(is_paid));
    }

    active_course.description = Set(body.description);
    active_course.background_image_url = Set(body.background_image_url);

    match active_course.update(db).await {
        Ok(updated_course) => HttpResponse::Ok().json(updated_course),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Update course error: {}", err)),
    }
}

pub async fn delete_course_service(
    db: &DatabaseConnection,
    course_id: i32,
) -> HttpResponse {
    match courses::Entity::delete_by_id(course_id).exec(db).await {
        Ok(result) => {
            if result.rows_affected == 0 {
                HttpResponse::NotFound().body("Course not found")
            } else {
                HttpResponse::Ok().body("Course deleted successfully")
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Delete course error: {}", err)),
    }
}


// Enrollment Admin CRUD
pub async fn get_all_enrollments(
    db: &DatabaseConnection,
) -> HttpResponse {
    match enrollments::Entity::find().all(db).await {
        Ok(enrollment_list) => HttpResponse::Ok().json(enrollment_list),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Database error: {}", err)),
    }
}

pub async fn admin_enroll_user_service(
    db: &DatabaseConnection,
    body: AdminEnrollmentForm,
) -> HttpResponse {
    // 1. Check if user exists
    let user_exists = match users::Entity::find_by_id(body.user_id)
        .one(db)
        .await
    {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("User check error: {}", err));
        }
    };

    if !user_exists {
        return HttpResponse::NotFound()
            .body("User not found");
    }

    // 2. Check if course exists
    let course_exists = match courses::Entity::find_by_id(body.course_id)
        .one(db)
        .await
    {
        Ok(Some(_)) => true,
        Ok(None) => false,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Course check error: {}", err));
        }
    };

    if !course_exists {
        return HttpResponse::NotFound()
            .body("Course not found");
    }

    // 3. Check if user is already enrolled
    match enrollments::Entity::find_by_id((body.user_id, body.course_id))
        .one(db)
        .await
    {
        Ok(Some(_)) => {
            return HttpResponse::BadRequest()
                .body("User is already enrolled in this course");
        }
        Ok(None) => {}
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Enrollment check error: {}", err));
        }
    }

    // 4. Create enrollment
    let new_enrollment = enrollments::ActiveModel {
        user_id: Set(body.user_id),
        course_id: Set(body.course_id),
        ..Default::default()
    };

    match new_enrollment.insert(db).await {
        Ok(enrollment) => HttpResponse::Ok().json(enrollment),
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Create enrollment error: {}", err)),
    }
}

pub async fn admin_unenroll_user_service(
    db: &DatabaseConnection,
    user_id: i32,
    course_id: i32,
) -> HttpResponse {
    match enrollments::Entity::delete_by_id((user_id, course_id))
        .exec(db)
        .await
    {
        Ok(result) => {
            if result.rows_affected == 0 {
                HttpResponse::NotFound().body("Enrollment not found")
            } else {
                HttpResponse::Ok().body("User unenrolled successfully")
            }
        }
        Err(err) => HttpResponse::InternalServerError()
            .body(format!("Delete enrollment error: {}", err)),
    }
}
