use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateOrganisationForm {
    #[validate(length(min = 1, message = "Organisation name is required"))]
    pub org_name: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateOrganisationForm {
    #[validate(length(min = 1, message = "Organisation name is required"))]
    pub org_name: String,
}

// User CRUD Forms
#[derive(Debug, Deserialize, Validate)]
pub struct CreateAdminUserForm {
    #[validate(length(min = 1, message = "First name is required"))]
    pub first_name: String,

    #[validate(length(min = 1, message = "Last name is required"))]
    pub last_name: String,

    #[validate(email(message = "Invalid email"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,

    pub org_id: Option<i32>,
    pub role_id: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateAdminUserForm {
    #[validate(length(min = 1, message = "First name is required"))]
    pub first_name: String,

    #[validate(length(min = 1, message = "Last name is required"))]
    pub last_name: String,

    #[validate(email(message = "Invalid email"))]
    pub email: String,

    pub org_id: Option<i32>,
}


// Course CRUD Forms
#[derive(Debug, Deserialize, Validate)]
pub struct CreateAdminCourseForm {
    #[validate(length(min = 1, message = "Course name is required"))]
    pub name: String,

    pub org_id: Option<i32>,
    pub instructor_id: Option<i32>,

    pub status: Option<String>,

    pub price_cents: Option<i32>,
    pub currency: Option<String>,
    pub is_paid: Option<bool>,

    pub description: Option<String>,
    pub background_image_url: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateAdminCourseForm {
    #[validate(length(min = 1, message = "Course name is required"))]
    pub name: String,

    pub org_id: Option<i32>,
    pub instructor_id: Option<i32>,

    pub status: Option<String>,

    pub price_cents: Option<i32>,
    pub currency: Option<String>,
    pub is_paid: Option<bool>,

    pub description: Option<String>,
    pub background_image_url: Option<String>,
}

// Enrollment Admin Forms
#[derive(Debug, Deserialize, Validate)]
pub struct AdminEnrollmentForm {
    pub user_id: i32,
    pub course_id: i32,
}


