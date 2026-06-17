use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    IntoActiveModel, QueryFilter, Set,
};
use sha1::{Digest, Sha1};
use uuid::Uuid;

use crate::entity::{password_reset_tokens, users};
use crate::services::mailer_service::{send_mail_message, MailRequest};
use crate::services::user_service::hash_password;

const TOKEN_EXPIRY_HOURS: i64 = 1;

#[derive(Debug)]
pub enum ResetPasswordError {
    InvalidOrExpired,
    GoogleAccount,
    Database(DbErr),
}

impl From<DbErr> for ResetPasswordError {
    fn from(err: DbErr) -> Self {
        ResetPasswordError::Database(err)
    }
}

pub fn reset_url(token: &str) -> String {
    let base_url =
        std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());
    format!(
        "{}/auth/reset-password?token={}",
        base_url.trim_end_matches('/'),
        token
    )
}

pub fn token_hash(token: &str) -> String {
    hex::encode(Sha1::digest(token.as_bytes()))
}

pub async fn create_password_reset_token<C>(db: &C, user_id: i32) -> Result<String, DbErr>
where
    C: ConnectionTrait,
{
    let token = format!("{}{}", Uuid::new_v4(), Uuid::new_v4());
    let hash = token_hash(&token);
    let expires_at = (Utc::now() + Duration::hours(TOKEN_EXPIRY_HOURS)).naive_utc();

    let new_token = password_reset_tokens::ActiveModel {
        user_id: Set(user_id),
        token_hash: Set(hash),
        expires_at: Set(expires_at),
        used_at: Set(None),
        ..Default::default()
    };

    new_token.insert(db).await?;

    Ok(token)
}

pub fn send_reset_email(email: &str, token: &str) -> Result<(), String> {
    let url = reset_url(token);
    let body = format!(
        "You requested a password reset for your SkillUp LMS account.\n\nReset your password by opening this link:\n{}\n\nThis link expires in {} hour(s). If you did not request this, you can safely ignore this email.",
        url, TOKEN_EXPIRY_HOURS
    );

    send_mail_message(MailRequest {
        to: email.to_string(),
        subject: "Reset your SkillUp LMS password".to_string(),
        body,
        is_html: false,
    })
}

/// Validates the token, applies the new password, marks the token used.
/// Returns the updated user on success.
pub async fn reset_password_with_token(
    db: &DatabaseConnection,
    token: &str,
    new_password: String,
) -> Result<users::Model, ResetPasswordError> {
    let hash = token_hash(token);

    let token_row = password_reset_tokens::Entity::find()
        .filter(password_reset_tokens::Column::TokenHash.eq(hash))
        .filter(password_reset_tokens::Column::UsedAt.is_null())
        .one(db)
        .await?
        .ok_or(ResetPasswordError::InvalidOrExpired)?;

    if token_row.expires_at < Utc::now().naive_utc() {
        return Err(ResetPasswordError::InvalidOrExpired);
    }

    let user = users::Entity::find_by_id(token_row.user_id)
        .one(db)
        .await?
        .ok_or(ResetPasswordError::InvalidOrExpired)?;

    // Block Google-only accounts from setting a password this way
    if user.auth_provider == "google" && user.password_hash.is_none() {
        return Err(ResetPasswordError::GoogleAccount);
    }

    let password_hash = hash_password(new_password)
        .await
        .map_err(|_| ResetPasswordError::Database(DbErr::Custom("Password hashing failed".into())))?;

    let mut active_user = user.into_active_model();
    active_user.password_hash = Set(Some(password_hash));
    active_user.auth_provider = Set("password".to_string());
    active_user.must_change_password = Set(false);
    let updated_user = active_user.update(db).await?;

    let mut active_token = token_row.into_active_model();
    active_token.used_at = Set(Some(Utc::now().naive_utc()));
    active_token.update(db).await?;

    Ok(updated_user)
}
