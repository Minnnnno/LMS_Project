use chrono::{DateTime, FixedOffset, Utc};
use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

pub const MAX_FAILED_LOGIN_ATTEMPTS: i32 = 5;
pub const LOCKOUT_MINUTES: i64 = 15;

pub fn is_locked(locked_until: Option<&DateTime<FixedOffset>>) -> bool {
    locked_until.is_some_and(|until| until > &Utc::now())
}

/// Atomically records a failed password attempt. Once the threshold is reached,
/// PostgreSQL starts a temporary lock. The first failure after an expired lock
/// starts a fresh counter instead of immediately locking the account again.
pub async fn record_failed_login<C>(db: &C, user_id: i32) -> Result<bool, DbErr>
where
    C: ConnectionTrait,
{
    let result = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"
        UPDATE users
        SET failed_login_attempts = CASE
                WHEN locked_until IS NOT NULL AND locked_until <= NOW() THEN 1
                ELSE failed_login_attempts + 1
            END,
            locked_until = CASE
                WHEN locked_until IS NOT NULL AND locked_until <= NOW() THEN NULL
                WHEN failed_login_attempts + 1 >= $2 THEN NOW() + ($3 * INTERVAL '1 minute')
                ELSE locked_until
            END,
            updated_at = NOW()
        WHERE user_id = $1
        RETURNING locked_until
        "#,
            [
                user_id.into(),
                MAX_FAILED_LOGIN_ATTEMPTS.into(),
                LOCKOUT_MINUTES.into(),
            ],
        ))
        .await?;

    let locked_until = result
        .and_then(|row| {
            row.try_get::<Option<DateTime<FixedOffset>>>("", "locked_until")
                .ok()
        })
        .flatten();

    Ok(is_locked(locked_until.as_ref()))
}

pub async fn clear_login_lockout<C>(db: &C, user_id: i32) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    db.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        r#"
        UPDATE users
        SET failed_login_attempts = 0,
            locked_until = NULL,
            updated_at = NOW()
        WHERE user_id = $1
          AND (failed_login_attempts <> 0 OR locked_until IS NOT NULL)
        "#,
        [user_id.into()],
    ))
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::is_locked;
    use chrono::{Duration, Utc};

    #[test]
    fn future_lock_is_active() {
        let locked_until = (Utc::now() + Duration::minutes(1)).fixed_offset();
        assert!(is_locked(Some(&locked_until)));
    }

    #[test]
    fn expired_or_missing_lock_is_inactive() {
        let expired = (Utc::now() - Duration::minutes(1)).fixed_offset();
        assert!(!is_locked(Some(&expired)));
        assert!(!is_locked(None));
    }
}
