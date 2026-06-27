use actix_session::SessionExt;
use actix_web::{
    Error,
    body::MessageBody,
    cookie::{Cookie, SameSite, time::Duration as CookieDuration},
    dev::{ServiceRequest, ServiceResponse},
    error::ErrorInternalServerError,
    middleware::Next,
    web,
};
use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, IntoActiveModel,
    QueryFilter, Set,
};
use sha1::{Digest, Sha1};
use uuid::Uuid;

use crate::config::is_production;
use crate::entity::{remember_me_tokens, users};
use crate::services::user_service::sign_user_into_session;

pub const REMEMBER_ME_COOKIE: &str = "remember_me";

const TOKEN_EXPIRY_DAYS: i64 = 30;

pub enum RememberMeValidation {
    Authenticated {
        user: users::Model,
        replacement_cookie: Cookie<'static>,
    },
    Invalid,
}

pub fn token_hash(token: &str) -> String {
    hex::encode(Sha1::digest(token.as_bytes()))
}

fn generate_token() -> String {
    format!("{}{}", Uuid::new_v4(), Uuid::new_v4())
}

fn remember_me_cookie(token: String) -> Cookie<'static> {
    Cookie::build(REMEMBER_ME_COOKIE, token)
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(is_production())
        .max_age(CookieDuration::days(TOKEN_EXPIRY_DAYS))
        .finish()
}

pub fn forget_remember_me_cookie() -> Cookie<'static> {
    Cookie::build(REMEMBER_ME_COOKIE, "")
        .path("/")
        .http_only(true)
        .same_site(SameSite::Lax)
        .secure(is_production())
        .max_age(CookieDuration::seconds(0))
        .finish()
}

pub async fn create_remember_me_cookie(
    db: &DatabaseConnection,
    user_id: i32,
    user_agent: Option<String>,
    ip_address: Option<String>,
) -> Result<Cookie<'static>, DbErr> {
    let token = generate_token();
    let hashed_token = token_hash(&token);
    let expires_at = (Utc::now() + Duration::days(TOKEN_EXPIRY_DAYS)).naive_utc();

    let new_token = remember_me_tokens::ActiveModel {
        user_id: Set(user_id),
        token_hash: Set(hashed_token),
        expires_at: Set(expires_at),
        last_used_at: Set(None),
        revoked_at: Set(None),
        user_agent: Set(user_agent),
        ip_address: Set(ip_address),
        ..Default::default()
    };

    new_token.insert(db).await?;

    Ok(remember_me_cookie(token))
}

pub async fn validate_and_rotate_remember_me_token(
    db: &DatabaseConnection,
    token: &str,
) -> Result<RememberMeValidation, DbErr> {
    let now = Utc::now().naive_utc();
    let hashed_token = token_hash(token);

    let token_row = match remember_me_tokens::Entity::find()
        .filter(remember_me_tokens::Column::TokenHash.eq(hashed_token))
        .filter(remember_me_tokens::Column::RevokedAt.is_null())
        .one(db)
        .await?
    {
        Some(token_row) => token_row,
        None => return Ok(RememberMeValidation::Invalid),
    };

    if token_row.expires_at < now {
        revoke_token_row(db, token_row).await?;
        return Ok(RememberMeValidation::Invalid);
    }

    let user = match users::Entity::find_by_id(token_row.user_id).one(db).await? {
        Some(user) => user,
        None => {
            revoke_token_row(db, token_row).await?;
            return Ok(RememberMeValidation::Invalid);
        }
    };

    let replacement_token = generate_token();
    let replacement_hash = token_hash(&replacement_token);
    let replacement_expires_at = (Utc::now() + Duration::days(TOKEN_EXPIRY_DAYS)).naive_utc();

    let mut active_token = token_row.into_active_model();
    active_token.token_hash = Set(replacement_hash);
    active_token.expires_at = Set(replacement_expires_at);
    active_token.last_used_at = Set(Some(now));
    active_token.update(db).await?;

    Ok(RememberMeValidation::Authenticated {
        user,
        replacement_cookie: remember_me_cookie(replacement_token),
    })
}

pub async fn revoke_remember_me_token(db: &DatabaseConnection, token: &str) -> Result<(), DbErr> {
    let hashed_token = token_hash(token);

    if let Some(token_row) = remember_me_tokens::Entity::find()
        .filter(remember_me_tokens::Column::TokenHash.eq(hashed_token))
        .filter(remember_me_tokens::Column::RevokedAt.is_null())
        .one(db)
        .await?
    {
        revoke_token_row(db, token_row).await?;
    }

    Ok(())
}

pub async fn remember_me_middleware(
    req: ServiceRequest,
    next: Next<impl MessageBody>,
) -> Result<ServiceResponse<impl MessageBody>, Error> {
    let mut response_cookie = None;
    let has_session = req
        .get_session()
        .get::<i32>("user_id")
        .ok()
        .flatten()
        .is_some();

    if !has_session {
        let remember_token = req
            .cookie(REMEMBER_ME_COOKIE)
            .map(|cookie| cookie.value().to_string());
        let db = req.app_data::<web::Data<DatabaseConnection>>().cloned();

        if let (Some(token), Some(db)) = (remember_token, db) {
            match validate_and_rotate_remember_me_token(db.get_ref(), &token).await {
                Ok(RememberMeValidation::Authenticated {
                    user,
                    replacement_cookie,
                }) => {
                    if let Err(message) =
                        sign_user_into_session(db.get_ref(), &req.get_session(), &user).await
                    {
                        println!("Remember-me session restore error: {}", message);
                        response_cookie = Some(forget_remember_me_cookie());
                    } else {
                        response_cookie = Some(replacement_cookie);
                    }
                }
                Ok(RememberMeValidation::Invalid) => {
                    response_cookie = Some(forget_remember_me_cookie());
                }
                Err(err) => {
                    println!("Remember-me token lookup error: {:?}", err);
                }
            }
        }
    }

    let mut response = next.call(req).await?;

    if let Some(cookie) = response_cookie {
        response
            .response_mut()
            .add_cookie(&cookie)
            .map_err(ErrorInternalServerError)?;
    }

    Ok(response)
}

async fn revoke_token_row(
    db: &DatabaseConnection,
    token_row: remember_me_tokens::Model,
) -> Result<(), DbErr> {
    let mut active_token = token_row.into_active_model();
    active_token.revoked_at = Set(Some(Utc::now().naive_utc()));
    active_token.update(db).await?;

    Ok(())
}
