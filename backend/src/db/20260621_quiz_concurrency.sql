BEGIN;

-- Keep the newest row if earlier concurrent saves created duplicate answers.
DELETE FROM quiz_answers older
USING quiz_answers newer
WHERE older.attempt_id = newer.attempt_id
  AND older.question_id = newer.question_id
  AND older.answer_id < newer.answer_id;

-- Keep the earliest open attempt if concurrent requests created more than one.
WITH ranked_open_attempts AS (
  SELECT attempt_id,
         ROW_NUMBER() OVER (
           PARTITION BY quiz_id, user_id
           ORDER BY started_at, attempt_id
         ) AS row_number
  FROM quiz_attempts
  WHERE submitted_at IS NULL
)
DELETE FROM quiz_attempts
WHERE attempt_id IN (
  SELECT attempt_id
  FROM ranked_open_attempts
  WHERE row_number > 1
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_quiz_attempts_one_open
  ON quiz_attempts (quiz_id, user_id)
  WHERE submitted_at IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_quiz_answers_attempt_question
  ON quiz_answers (attempt_id, question_id);

COMMIT;
