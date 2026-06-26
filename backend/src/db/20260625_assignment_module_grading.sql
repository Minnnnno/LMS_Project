-- Link assignments to modules and support assignment passing marks.

ALTER TABLE assignments
ADD COLUMN IF NOT EXISTS module_id INTEGER;

ALTER TABLE assignments
ADD COLUMN IF NOT EXISTS passing_mark NUMERIC(5,2) NOT NULL DEFAULT 50;

-- Existing installations may have course-level assignments without modules.
-- Create one fallback module per affected course before making module_id mandatory.
INSERT INTO modules (course_id, title, position)
SELECT a.course_id, 'General Assignments', COALESCE(MAX(m.position), 0) + 1
FROM assignments a
LEFT JOIN modules m ON m.course_id = a.course_id
WHERE a.module_id IS NULL
GROUP BY a.course_id
HAVING COUNT(m.module_id) = 0;

WITH first_module AS (
    SELECT DISTINCT ON (course_id)
        course_id,
        module_id
    FROM modules
    ORDER BY course_id, position, module_id
)
UPDATE assignments a
SET module_id = fm.module_id
FROM first_module fm
WHERE a.course_id = fm.course_id
  AND a.module_id IS NULL;

ALTER TABLE assignments
ALTER COLUMN module_id SET NOT NULL;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'assignments_module_id_fkey'
    ) THEN
        ALTER TABLE assignments
        ADD CONSTRAINT assignments_module_id_fkey
        FOREIGN KEY (module_id)
        REFERENCES modules(module_id)
        ON DELETE CASCADE;
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'assignments_passing_mark_range'
    ) THEN
        ALTER TABLE assignments
        ADD CONSTRAINT assignments_passing_mark_range
        CHECK (passing_mark >= 0 AND passing_mark <= 100);
    END IF;
END $$;

CREATE INDEX IF NOT EXISTS idx_assignments_course_id
ON assignments(course_id);

CREATE INDEX IF NOT EXISTS idx_assignments_module_id
ON assignments(module_id);

CREATE INDEX IF NOT EXISTS idx_assignments_due_date
ON assignments(due_date);

CREATE INDEX IF NOT EXISTS idx_quizzes_course_id
ON quizzes(course_id);

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'quizzes_passing_mark_range'
    ) THEN
        ALTER TABLE quizzes
        ADD CONSTRAINT quizzes_passing_mark_range
        CHECK (passing_mark >= 0 AND passing_mark <= 100);
    END IF;
END $$;

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'quizzes_time_limit_positive'
    ) THEN
        ALTER TABLE quizzes
        ADD CONSTRAINT quizzes_time_limit_positive
        CHECK (time_limit IS NULL OR time_limit > 0);
    END IF;
END $$;
