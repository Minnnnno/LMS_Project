use crate::models::validation::validate_password_complexity;
use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterForm {
    #[validate(length(min = 1, message = "First name is required."))]
    pub first_name: String,

    #[validate(length(min = 1, message = "Last name is required."))]
    pub last_name: String,

    #[validate(email(message = "Please enter a valid email address."))]
    pub email: String,

    #[validate(
        length(min = 8, max = 128, message = "Password must be between 8 and 128 characters."),
        custom = "validate_password_complexity"
    )]
    pub password: String,

    #[validate(length(
        min = 8,
        max = 128,
        message = "Confirm password must be between 8 and 128 characters."
    ))]
    pub confirm_password: String,

    #[serde(rename = "g-recaptcha-response")]
    pub recaptcha_response: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct LoginForm {
    #[validate(email(message = "Please enter a valid email address."))]
    pub email: String,

    #[validate(length(min = 1, message = "Password is required."))]
    pub password: String,

    pub remember_me: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ForgotPasswordForm {
    #[validate(email(message = "Please enter a valid email address."))]
    pub email: String,
}

#[derive(Debug, Deserialize, Validate)]
pub struct ResetPasswordForm {
    #[validate(length(min = 1, message = "Token is required."))]
    pub token: String,

    #[validate(
        length(min = 8, max = 128, message = "Password must be between 8 and 128 characters."),
        custom = "validate_password_complexity"
    )]
    pub password: String,

    #[validate(length(
        min = 8,
        max = 128,
        message = "Confirm password must be between 8 and 128 characters."
    ))]
    pub confirm_password: String,
}
