-- Run once against an existing SkillUp LMS PostgreSQL database.
ALTER TABLE users
ADD COLUMN IF NOT EXISTS failed_login_attempts INT NOT NULL DEFAULT 0;

ALTER TABLE users
ADD COLUMN IF NOT EXISTS locked_until TIMESTAMPTZ;

ALTER TABLE users
DROP CONSTRAINT IF EXISTS users_failed_login_attempts_nonnegative;

ALTER TABLE users
ADD CONSTRAINT users_failed_login_attempts_nonnegative
CHECK (failed_login_attempts >= 0);
