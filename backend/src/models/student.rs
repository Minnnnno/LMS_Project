use crate::models::validation::validate_password_complexity;
use serde::Deserialize;
use validator::Validate;

//form for updating own profile
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateOwnProfileForm {
    #[validate(length(min = 1, message = "First name is required"))]
    pub first_name: String,

    #[validate(length(min = 1, message = "Last name is required"))]
    pub last_name: String,

    #[validate(email(message = "Invalid email"))]
    pub email: String,
}

//for password changing form
#[derive(Debug, Deserialize, Validate)]
pub struct ChangePasswordForm {
    #[validate(length(min = 1, message = "Current password is required"))]
    pub current_password: String,

    #[validate(
        length(min = 8, max = 128, message = "New password must be between 8 and 128 characters."),
        custom = "validate_password_complexity"
    )]
    pub new_password: String,

    #[validate(length(
        min = 8,
        max = 128,
        message = "Confirm password must be between 8 and 128 characters"
    ))]
    pub confirm_password: String,
}
