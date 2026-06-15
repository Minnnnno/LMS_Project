use actix_web::HttpResponse;
use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter, Set,
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
    roles,
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

fn singapore_now() -> DateTime<FixedOffset> {
    let singapore_offset = FixedOffset::east_opt(8 * 60 * 60)
        .expect("Singapore UTC offset must be valid");

    Utc::now().with_timezone(&singapore_offset)
}

fn required_trimmed(value: &str, field_name: &str) -> Result<String, HttpResponse> {
    let value = value.trim();

    if value.is_empty() {
        Err(HttpResponse::BadRequest().body(format!("{} is required", field_name)))
    } else {
        Ok(value.to_string())
    }
}

async fn email_is_used_by_another_user(
    db: &DatabaseConnection,
    email: &str,
    excluded_user_id: Option<i32>,
) -> Result<bool, HttpResponse> {
    let mut query = users::Entity::find().filter(users::Column::Email.eq(email));

    if let Some(user_id) = excluded_user_id {
        query = query.filter(users::Column::UserId.ne(user_id));
    }

    query
        .one(db)
        .await
        .map(|user| user.is_some())
        .map_err(|err| HttpResponse::InternalServerError().body(format!("Email lookup error: {}", err)))
}

fn validate_optional_website_url(website_url: Option<&str>) -> Result<(), HttpResponse> {
    let Some(website_url) = website_url.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };

    if website_url.starts_with("https://") || website_url.starts_with("http://") {
        Ok(())
    } else {
        Err(HttpResponse::BadRequest().body("Website URL must start with http:// or https://"))
    }
}

fn validate_course_payment(
    is_paid: Option<bool>,
    price_cents: Option<i32>,
    currency: Option<&str>,
) -> Result<(), HttpResponse> {
    if price_cents.is_some_and(|price| price < 0) {
        return Err(HttpResponse::BadRequest().body("Course price cannot be negative"));
    }

    if is_paid.unwrap_or(false) {
        if !price_cents.is_some_and(|price| price > 0) {
            return Err(HttpResponse::BadRequest().body("Paid courses must have a price greater than zero"));
        }

        if !currency.is_some_and(|value| value.eq_ignore_ascii_case("SGD")) {
            return Err(HttpResponse::BadRequest().body("Paid courses currently support SGD only"));
        }
    } else if price_cents.is_some() || currency.is_some() {
        return Err(HttpResponse::BadRequest()
            .body("Unpaid courses must not have a price or currency"));
    }

    Ok(())
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

pub async fn get_all_roles(
    db: &DatabaseConnection,
) -> HttpResponse {
    match roles::Entity::find().all(db).await {
        Ok(role_list) => HttpResponse::Ok().json(role_list),
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

    let org_name = match required_trimmed(&body.org_name, "Organisation name") {
        Ok(name) => name,
        Err(response) => return response,
    };
    let now = singapore_now();
    let new_org = organisations::ActiveModel {
        org_name: Set(org_name),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
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

    let org_name = match required_trimmed(&body.org_name, "Organisation name") {
        Ok(name) => name,
        Err(response) => return response,
    };
    let mut active_org = org.into_active_model();
    active_org.org_name = Set(org_name);
    if let Some(org_slug) = body.org_slug {
        active_org.org_slug = Set({
            let value = org_slug.trim().to_lowercase();
            (!value.is_empty()).then_some(value)
        });
    }
    if let Err(response) = validate_optional_website_url(body.website_url.as_deref()) {
        return response;
    }
    if let Some(org_type) = body.org_type {
        active_org.org_type = Set({
            let value = org_type.trim().to_string();
            (!value.is_empty()).then_some(value)
        });
    }
    if let Some(website_url) = body.website_url {
        active_org.website_url = Set({
            let value = website_url.trim().to_string();
            (!value.is_empty()).then_some(value)
        });
    }
    active_org.updated_at = Set(Some(singapore_now()));

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

    let first_name = match required_trimmed(&body.first_name, "First name") {
        Ok(name) => name,
        Err(response) => return response,
    };
    let last_name = match required_trimmed(&body.last_name, "Last name") {
        Ok(name) => name,
        Err(response) => return response,
    };
    let email = body.email.trim().to_lowercase();

    match email_is_used_by_another_user(db, &email, None).await {
        Ok(true) => return HttpResponse::Conflict().body("A user with this email already exists"),
        Ok(false) => {}
        Err(response) => return response,
    }

    let password_hash = match hash_password(&body.password) {
        Ok(hash) => hash,
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Password hash error: {}", err));
        }
    };

    let role_id = body.role_id;
    let now = singapore_now();

    let new_user = users::ActiveModel {
        first_name: Set(first_name),
        last_name: Set(last_name),
        email: Set(email),
        password_hash: Set(Some(password_hash)),
        org_id: Set(body.org_id),
        email_verified: Set(false),
        must_change_password: Set(false),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
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

    let first_name = match required_trimmed(&body.first_name, "First name") {
        Ok(name) => name,
        Err(response) => return response,
    };
    let last_name = match required_trimmed(&body.last_name, "Last name") {
        Ok(name) => name,
        Err(response) => return response,
    };
    let email = body.email.trim().to_lowercase();

    match email_is_used_by_another_user(db, &email, Some(user_id)).await {
        Ok(true) => return HttpResponse::Conflict().body("A user with this email already exists"),
        Ok(false) => {}
        Err(response) => return response,
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

    active_user.first_name = Set(first_name);
    active_user.last_name = Set(last_name);
    active_user.email = Set(email);
    active_user.org_id = Set(body.org_id);
    active_user.updated_at = Set(Some(singapore_now()));

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
    if let Err(response) = validate_course_payment(body.is_paid, body.price_cents, body.currency.as_deref()) {
        return response;
    }

    let status = match parse_course_status(body.status) {
        Ok(status) => status,
        Err(response) => return response,
    };
    let course_name = match required_trimmed(&body.name, "Course name") {
        Ok(name) => name,
        Err(response) => return response,
    };
    let now = singapore_now();

    let new_course = courses::ActiveModel {
        name: Set(Some(course_name)),
        org_id: Set(body.org_id),
        instructor_id: Set(body.instructor_id),
        status: Set(status),
        price_cents: Set(body.price_cents),
        currency: Set(body.currency),
        is_paid: Set(Some(body.is_paid.unwrap_or(false))),
        description: Set(body.description),
        background_image_url: Set(body.background_image_url),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
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
    if let Err(response) = validate_course_payment(body.is_paid, body.price_cents, body.currency.as_deref()) {
        return response;
    }

    let course = match courses::Entity::find_by_id(course_id).one(db).await {
        Ok(Some(course)) => course,
        Ok(None) => return HttpResponse::NotFound().body("Course not found"),
        Err(err) => {
            return HttpResponse::InternalServerError()
                .body(format!("Database error: {}", err));
        }
    };

    let course_name = match required_trimmed(&body.name, "Course name") {
        Ok(name) => name,
        Err(response) => return response,
    };
    let mut active_course = course.into_active_model();

    active_course.name = Set(Some(course_name));
    active_course.org_id = Set(body.org_id);
    active_course.instructor_id = Set(body.instructor_id);

    if body.status.is_some() {
        let status = match parse_course_status(body.status) {
            Ok(status) => status,
            Err(response) => return response,
        };

        active_course.status = Set(status);
    }

    if let Some(is_paid) = body.is_paid {
        active_course.is_paid = Set(Some(is_paid));

        if is_paid {
            active_course.price_cents = Set(body.price_cents);
            active_course.currency = Set(body.currency);
        } else {
            active_course.price_cents = Set(None);
            active_course.currency = Set(None);
        }
    }

    active_course.description = Set(body.description);
    active_course.background_image_url = Set(body.background_image_url);
    active_course.updated_at = Set(Some(singapore_now()));

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
    let now = singapore_now();
    let new_enrollment = enrollments::ActiveModel {
        user_id: Set(body.user_id),
        course_id: Set(body.course_id),
        created_at: Set(Some(now)),
        updated_at: Set(Some(now)),
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
