# Demo user seed

This seed creates only user-related data:

- `organisations`
- `users`
- `roles`
- `user_roles`

It does not insert courses, modules, assignments, enrollments, payments, or learning progress.

## Run the seed

From `backend`:

```powershell
cargo run --bin seed_demo_users
```

The seed loads `backend/.env`, connects with `DATABASE_URL`, creates missing users through
`create_user_service`, and uses `Password123!` for every newly created account.

Existing users are matched by email and are not recreated. Their `org_id`, role, verification
state, password-change flag, and lockout fields are refreshed for the demo dataset.

## Print the SQL sections

From `backend`:

```powershell
cargo run --bin seed_demo_users -- --print-sql
```

This prints:

1. Organisation and role seed SQL
2. SQL to assign `org_id`
3. SQL to assign roles
4. Summary counts

The printed SQL intentionally does not insert users or password hashes. User creation stays in
Rust so Argon2 hashing, validation, and service logic remain the source of truth.

## Summary

- Organisations: 10
- LMS Admin: 1
- Organisation Admins: 10
- Instructors: 50
- Organisation Students: 100
- Public Students: 20
- Total users: 181

Public students use `student.skillup.com` email addresses and keep `org_id = NULL`.
