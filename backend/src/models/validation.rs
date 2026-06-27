use validator::ValidationError;

pub fn validate_password_complexity(password: &str) -> Result<(), ValidationError> {
    let has_uppercase = password.chars().any(|c| c.is_ascii_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| !c.is_ascii_alphanumeric());

    if !has_uppercase || !has_lowercase || !has_digit || !has_special {
        let mut error = ValidationError::new("password_complexity");
        error.message = Some(
            "Password must contain at least one uppercase letter, one lowercase letter, one number, and one special character.".into()
        );
        return Err(error);
    }

    Ok(())
}
