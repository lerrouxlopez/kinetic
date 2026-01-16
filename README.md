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

## Deployment (RunCloud + GHCR)

This repo includes a Dockerfile and a GitHub Actions workflow that builds and
pushes `ghcr.io/<owner>/kinetic:latest` on each push to `master`, then SSHes into
your server to pull and restart the container.

### Server setup

Create `/opt/kinetic/docker-compose.yml` on the VPS:

```yaml
services:
  app:
    image: ghcr.io/<owner>/kinetic:latest
    restart: unless-stopped
    ports:
      - "127.0.0.1:8000:8000"
    environment:
      ROCKET_ADDRESS: 0.0.0.0
      ROCKET_PORT: 8000
      KINETIC_ADMIN_EMAIL: you@example.com
      KINETIC_ADMIN_PASSWORD: strong-password
    volumes:
      - /opt/kinetic/data/kinetic.db:/app/kinetic.db
```

Then point RunCloud's Nginx vhost to `http://127.0.0.1:8000`.

Example Nginx location block:

```nginx
location / {
  proxy_pass http://127.0.0.1:8000;
  proxy_set_header Host $host;
  proxy_set_header X-Real-IP $remote_addr;
  proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
  proxy_set_header X-Forwarded-Proto $scheme;
}
```

### GitHub Secrets

Add these repository secrets:

- `SERVER_HOST`
- `SERVER_USER`
- `SERVER_SSH_KEY`
- optional: `SERVER_PORT`

### First deploy checklist

1. Create the `/opt/kinetic` folder and `docker-compose.yml`.
2. Make sure Docker and Compose are installed on the server.
3. Add the GitHub secrets above.
4. Push to `master` and confirm the workflow run succeeds.
5. Visit your domain and complete the initial admin login.

### Health check and rollback

- Health check: add a lightweight route (like `/health`) that returns `200 OK`,
  then point your uptime monitor to it.
- Rollback: change `docker-compose.yml` to the previous tag (or `latest` minus
  one build), then run `docker compose up -d`.

## Notes

- Templates live in `templates/`.
- Static assets live in `static/`.
