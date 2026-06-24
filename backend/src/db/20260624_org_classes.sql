CREATE TABLE IF NOT EXISTS org_classes (
  class_id   SERIAL PRIMARY KEY,
  org_id     INT NOT NULL,
  class_name VARCHAR(255) NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE (org_id, class_name),
  FOREIGN KEY (org_id)
    REFERENCES organisations(org_id)
    ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_org_classes_org_id
  ON org_classes(org_id);

CREATE TABLE IF NOT EXISTS org_class_courses (
  class_id    INT NOT NULL,
  course_id   INT NOT NULL,
  assigned_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
  PRIMARY KEY (class_id, course_id),
  FOREIGN KEY (class_id)
    REFERENCES org_classes(class_id)
    ON DELETE CASCADE,
  FOREIGN KEY (course_id)
    REFERENCES courses(course_id)
    ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_org_class_courses_course_id
  ON org_class_courses(course_id);

-- Backfill databases that briefly used org_classes.course_id before
-- class-course many-to-many support was added.
DO $$
BEGIN
  IF EXISTS (
    SELECT 1
    FROM information_schema.columns
    WHERE table_name = 'org_classes'
      AND column_name = 'course_id'
  ) THEN
    INSERT INTO org_class_courses (class_id, course_id)
    SELECT class_id, course_id
    FROM org_classes
    WHERE course_id IS NOT NULL
    ON CONFLICT DO NOTHING;

    ALTER TABLE org_classes
    ALTER COLUMN course_id DROP NOT NULL;

    ALTER TABLE org_classes
    DROP COLUMN IF EXISTS course_id;
  END IF;
END $$;

CREATE TABLE IF NOT EXISTS org_class_members (
  class_id    INT NOT NULL,
  user_id     INT NOT NULL,
  assigned_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
  assigned_by INT,
  PRIMARY KEY (class_id, user_id),
  FOREIGN KEY (class_id)
    REFERENCES org_classes(class_id)
    ON DELETE CASCADE,
  FOREIGN KEY (user_id)
    REFERENCES users(user_id)
    ON DELETE CASCADE,
  FOREIGN KEY (assigned_by)
    REFERENCES users(user_id)
    ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_org_class_members_user_id
  ON org_class_members(user_id);
