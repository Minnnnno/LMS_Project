# Demo course seed

Run the schema update before seeding courses:

```powershell
psql "<DATABASE_URL>" -f backend/src/db/20260625_assignment_module_grading.sql
```

Then run the user seed if the target database does not already have the demo users:

```powershell
cd backend
cargo run --bin seed_demo_users
```

Finally seed the course structure:

```powershell
cargo run --bin seed_demo_courses
```

The course seed expects the demo organisations, instructors, and students to exist. It creates:

- 25 courses
- 125 modules
- 100 module prerequisite links
- 625 module content records
- 175 module-linked assignments
- 25 quizzes
- 250 quiz questions
- 700 quiz options
- Enrollments and module progress records for demo learners

The seed rebuilds child data for the seeded courses on rerun while keeping course rows stable where possible.
