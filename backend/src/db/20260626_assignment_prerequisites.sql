CREATE TABLE IF NOT EXISTS assignment_prerequisites (
  prerequisite_id    SERIAL PRIMARY KEY,
  assignment_id      INT NOT NULL,
  required_module_id INT NOT NULL,
  created_at         TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_assignment_prerequisites_assignment
    FOREIGN KEY (assignment_id)
    REFERENCES assignments(assignment_id)
    ON DELETE CASCADE,
  CONSTRAINT fk_assignment_prerequisites_required_module
    FOREIGN KEY (required_module_id)
    REFERENCES modules(module_id)
    ON DELETE CASCADE,
  CONSTRAINT uq_assignment_prerequisite_module
    UNIQUE (assignment_id, required_module_id)
);

CREATE INDEX IF NOT EXISTS idx_assignment_prerequisites_assignment_id
  ON assignment_prerequisites(assignment_id);

CREATE INDEX IF NOT EXISTS idx_assignment_prerequisites_required_module_id
  ON assignment_prerequisites(required_module_id);
