use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    IntoActiveModel, QueryFilter, Set,
};
use sha1::{Digest, Sha1};
use uuid::Uuid;

use crate::entity::{email_verification_tokens, users};
use crate::services::mailer_service::{send_mail_message, MailRequest};

const TOKEN_EXPIRY_HOURS: i64 = 24;

#[derive(Debug)]
pub enum VerifyEmailError {
    InvalidOrExpired,
    Database(DbErr),
}

impl From<DbErr> for VerifyEmailError {
    fn from(err: DbErr) -> Self {
        VerifyEmailError::Database(err)
    }
}

pub fn verification_url(token: &str) -> String {
    let base_url =
        std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_string());

    format!("{}/auth/verify-email?token={}", base_url.trim_end_matches('/'), token)
}

pub fn token_hash(token: &str) -> String {
    hex::encode(Sha1::digest(token.as_bytes()))
}

pub async fn create_email_verification_token<C>(db: &C, user_id: i32) -> Result<String, DbErr>
where
    C: ConnectionTrait,
{
    let token = format!("{}{}", Uuid::new_v4(), Uuid::new_v4());
    let token_hash = token_hash(&token);
    let expires_at = (Utc::now() + Duration::hours(TOKEN_EXPIRY_HOURS)).naive_utc();

    let new_token = email_verification_tokens::ActiveModel {
        user_id: Set(user_id),
        token_hash: Set(token_hash),
        expires_at: Set(expires_at),
        used_at: Set(None),
        ..Default::default()
    };

    new_token.insert(db).await?;

    Ok(token)
}

pub fn send_verification_email(email: &str, token: &str) -> Result<(), String> {
    let verify_url = verification_url(token);
    let body = format!(
        "Welcome to SkillUp LMS.\n\nVerify your email address by opening this link:\n{}\n\nThis link expires in {} hours.",
        verify_url, TOKEN_EXPIRY_HOURS
    );

    send_mail_message(MailRequest {
        to: email.to_string(),
        subject: "Verify your SkillUp LMS email".to_string(),
        body,
        is_html: false,
    })
}

pub async fn verify_email_token(
    db: &DatabaseConnection,
    token: &str,
) -> Result<users::Model, VerifyEmailError> {
    let token_hash = token_hash(token);
    let token_row = email_verification_tokens::Entity::find()
        .filter(email_verification_tokens::Column::TokenHash.eq(token_hash))
        .filter(email_verification_tokens::Column::UsedAt.is_null())
        .one(db)
        .await?
        .ok_or(VerifyEmailError::InvalidOrExpired)?;

    if token_row.expires_at < Utc::now().naive_utc() {
        return Err(VerifyEmailError::InvalidOrExpired);
    }

    let user = users::Entity::find_by_id(token_row.user_id)
        .one(db)
        .await?
        .ok_or(VerifyEmailError::InvalidOrExpired)?;

    let mut active_user = user.into_active_model();
    active_user.email_verified = Set(true);
    let verified_user = active_user.update(db).await?;

    let mut active_token = token_row.into_active_model();
    active_token.used_at = Set(Some(Utc::now().naive_utc()));
    active_token.update(db).await?;

    Ok(verified_user)
}
