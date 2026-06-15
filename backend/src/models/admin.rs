use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateOrganisationForm {
    #[validate(length(min = 1, max = 255, message = "Organisation name must be between 1 and 255 characters"))]
    pub org_name: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateOrganisationForm {
    #[validate(length(min = 1, max = 255, message = "Organisation name must be between 1 and 255 characters"))]
    pub org_name: String,

    pub org_slug: Option<String>,
    pub org_type: Option<String>,
    pub website_url: Option<String>,
}

// User CRUD Forms
#[derive(Debug, Deserialize, Validate)]
pub struct CreateAdminUserForm {
    #[validate(length(min = 1, max = 100, message = "First name must be between 1 and 100 characters"))]
    pub first_name: String,

    #[validate(length(min = 1, max = 100, message = "Last name must be between 1 and 100 characters"))]
    pub last_name: String,

    #[validate(email(message = "Enter a valid email address"))]
    #[validate(length(max = 255, message = "Email must not exceed 255 characters"))]
    pub email: String,

    #[validate(length(min = 8, message = "Password must be at least 8 characters"))]
    pub password: String,

    pub org_id: Option<i32>,
    pub role_id: Option<i32>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateAdminUserForm {
    #[validate(length(min = 1, max = 100, message = "First name must be between 1 and 100 characters"))]
    pub first_name: String,

    #[validate(length(min = 1, max = 100, message = "Last name must be between 1 and 100 characters"))]
    pub last_name: String,

    #[validate(email(message = "Enter a valid email address"))]
    #[validate(length(max = 255, message = "Email must not exceed 255 characters"))]
    pub email: String,

    pub org_id: Option<i32>,
}


// Course CRUD Forms
#[derive(Debug, Deserialize, Validate)]
pub struct CreateAdminCourseForm {
    #[validate(length(min = 1, max = 255, message = "Course name must be between 1 and 255 characters"))]
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
    #[validate(length(min = 1, max = 255, message = "Course name must be between 1 and 255 characters"))]
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


