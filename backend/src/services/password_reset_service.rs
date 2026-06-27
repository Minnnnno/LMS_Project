use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbBackend, DbErr,
    EntityTrait, IntoActiveModel, QueryFilter, Set, Statement, TransactionTrait,
};
use sha1::{Digest, Sha1};
use uuid::Uuid;

use crate::entity::{password_reset_tokens, users};
use crate::services::mailer_service::{MailRequest, send_mail_message};
use crate::services::user_service::hash_password;

const TOKEN_EXPIRY_MINUTES: i64 = 30;

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

pub async fn create_password_reset_token(
    db: &DatabaseConnection,
    user_id: i32,
) -> Result<String, DbErr> {
    let transaction = db.begin().await?;
    let token = format!("{}{}", Uuid::new_v4(), Uuid::new_v4());
    let hash = token_hash(&token);
    let expires_at = (Utc::now() + Duration::minutes(TOKEN_EXPIRY_MINUTES)).naive_utc();

    // Only the newest reset email should remain valid.
    password_reset_tokens::Entity::update_many()
        .col_expr(
            password_reset_tokens::Column::UsedAt,
            sea_orm::sea_query::Expr::value(Utc::now().naive_utc()),
        )
        .filter(password_reset_tokens::Column::UserId.eq(user_id))
        .filter(password_reset_tokens::Column::UsedAt.is_null())
        .exec(&transaction)
        .await?;

    let new_token = password_reset_tokens::ActiveModel {
        user_id: Set(user_id),
        token_hash: Set(hash),
        expires_at: Set(expires_at),
        used_at: Set(None),
        ..Default::default()
    };

    new_token.insert(&transaction).await?;
    transaction.commit().await?;

    Ok(token)
}

pub fn send_reset_email(email: &str, token: &str) -> Result<(), String> {
    let url = reset_url(token);
    let body = format!(
        "You requested a password reset for your SkillUp LMS account.\n\nReset your password by opening this link:\n{}\n\nThis link expires in {} minutes. If you did not request this, you can safely ignore this email.",
        url, TOKEN_EXPIRY_MINUTES
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
    let transaction = db.begin().await?;
    let hash = token_hash(token);

    // Claim the token atomically so simultaneous submissions cannot reuse it.
    let claimed_token = transaction
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"
            UPDATE password_reset_tokens
            SET used_at = NOW()
            WHERE token_hash = $1
              AND used_at IS NULL
              AND expires_at > NOW()
            RETURNING user_id
            "#,
            [hash.into()],
        ))
        .await?;
    let user_id = claimed_token
        .and_then(|row| row.try_get::<i32>("", "user_id").ok())
        .ok_or(ResetPasswordError::InvalidOrExpired)?;

    let user = users::Entity::find_by_id(user_id)
        .one(&transaction)
        .await?
        .ok_or(ResetPasswordError::InvalidOrExpired)?;

    // Block Google-only accounts from setting a password this way
    if user.auth_provider == "google" && user.password_hash.is_none() {
        return Err(ResetPasswordError::GoogleAccount);
    }

    let password_hash = hash_password(new_password).await.map_err(|_| {
        ResetPasswordError::Database(DbErr::Custom("Password hashing failed".into()))
    })?;

    let mut active_user = user.into_active_model();
    active_user.password_hash = Set(Some(password_hash));
    active_user.auth_provider = Set("password".to_string());
    active_user.must_change_password = Set(false);
    active_user.failed_login_attempts = Set(0);
    active_user.locked_until = Set(None);
    let updated_user = active_user.update(&transaction).await?;

    // A successful reset invalidates every outstanding link for the account.
    password_reset_tokens::Entity::update_many()
        .col_expr(
            password_reset_tokens::Column::UsedAt,
            sea_orm::sea_query::Expr::value(Utc::now().naive_utc()),
        )
        .filter(password_reset_tokens::Column::UserId.eq(updated_user.user_id))
        .filter(password_reset_tokens::Column::UsedAt.is_null())
        .exec(&transaction)
        .await?;

    transaction.commit().await?;

    Ok(updated_user)
}
