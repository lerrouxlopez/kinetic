# Kinetic

Asset + manpower tracking app built with Rocket (Rust) and SQLite, designed to evolve into a multi-tenant MySQL setup.

## Quick start

1. Update the `secret_key` in `Rocket.toml`.
2. Run the app:

```bash
cargo run
```

3. Visit `http://127.0.0.1:8000`.

The first registration creates a workspace (tenant). Login is scoped by workspace slug + email.

## Super admin

On first boot, a super admin is seeded if none exists.

- Default email: `admin@kinetic.local`
- Default password: `ChangeMe123!`

Override with environment variables before running:

```bash
set KINETIC_ADMIN_EMAIL=you@example.com
set KINETIC_ADMIN_PASSWORD=strong-password
```

Admin UI: `http://127.0.0.1:8000/admin/login`

## Storage

- Current: SQLite (`kinetic.db` in the project root).
- Migration-ready: dependencies include `sqlx_mysql` so you can swap the database URL when ready.

When moving to MySQL:

1. Update `Rocket.toml` to `mysql://user:pass@host/db`.
2. Replace the SQLite schema setup with migrations or MySQL-specific DDL.

## Notes

- Templates live in `templates/`.
- Static assets live in `static/`.
