use rocket_db_pools::sqlx::{self, Row};
use crate::repositories::{tenant_repo, user_repo};
use crate::services::utils::hash_password;
use crate::Db;

pub async fn ensure_schema(db: &Db) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
          CREATE TABLE IF NOT EXISTS tenants (
              id INTEGER PRIMARY KEY AUTOINCREMENT,
              slug TEXT NOT NULL UNIQUE,
              name TEXT NOT NULL,
              app_name TEXT NOT NULL DEFAULT 'Kinetic',
              logo_path TEXT NOT NULL DEFAULT '',
              theme_key TEXT NOT NULL DEFAULT 'kinetic',
              background_hue INTEGER NOT NULL DEFAULT 32,
              body_font TEXT NOT NULL DEFAULT 'Space Grotesk',
              heading_font TEXT NOT NULL DEFAULT 'Space Grotesk',
              plan_key TEXT NOT NULL DEFAULT 'free',
              plan_started_at TEXT NOT NULL DEFAULT (datetime('now')),
              plan_expired INTEGER NOT NULL DEFAULT 0,
              email_provider TEXT NOT NULL DEFAULT 'Mailtrap',
              email_from_name TEXT NOT NULL DEFAULT '',
              email_from_address TEXT NOT NULL DEFAULT '',
            smtp_host TEXT NOT NULL DEFAULT '',
            smtp_port TEXT NOT NULL DEFAULT '',
            smtp_username TEXT NOT NULL DEFAULT '',
            smtp_password TEXT NOT NULL DEFAULT '',
            smtp_encryption TEXT NOT NULL DEFAULT '',
            mailgun_domain TEXT NOT NULL DEFAULT '',
            mailgun_api_key TEXT NOT NULL DEFAULT '',
            postmark_server_token TEXT NOT NULL DEFAULT '',
            resend_api_key TEXT NOT NULL DEFAULT '',
            ses_access_key TEXT NOT NULL DEFAULT '',
            ses_secret_key TEXT NOT NULL DEFAULT '',
            ses_region TEXT NOT NULL DEFAULT '',
            sendmail_path TEXT NOT NULL DEFAULT ''
        )
        "#,
    )
    .execute(&db.0)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tenant_id INTEGER NOT NULL,
            is_super_admin INTEGER NOT NULL DEFAULT 0,
            email TEXT NOT NULL,
            password_hash TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'Owner',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(tenant_id, email),
            FOREIGN KEY(tenant_id) REFERENCES tenants(id)
        )
        "#,
    )
    .execute(&db.0)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS admins (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )
        "#,
    )
    .execute(&db.0)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS crews (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tenant_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            members_count INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'Active',
            gear_score INTEGER NOT NULL DEFAULT 100,
            skill_tags TEXT NOT NULL DEFAULT '',
            compatibility_tags TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY(tenant_id) REFERENCES tenants(id)
        )
        "#,
    )
    .execute(&db.0)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS crew_members (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            crew_id INTEGER NOT NULL,
            tenant_id INTEGER NOT NULL,
            user_id INTEGER,
            name TEXT NOT NULL,
            phone TEXT NOT NULL,
            email TEXT NOT NULL,
            position TEXT NOT NULL,
            availability_status TEXT NOT NULL DEFAULT 'Available',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY(crew_id) REFERENCES crews(id) ON DELETE CASCADE,
            FOREIGN KEY(tenant_id) REFERENCES tenants(id)
        )
        "#,
    )
    .execute(&db.0)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS clients (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tenant_id INTEGER NOT NULL,
            company_name TEXT NOT NULL,
            address TEXT NOT NULL,
            phone TEXT NOT NULL,
            email TEXT NOT NULL,
            latitude TEXT NOT NULL,
            longitude TEXT NOT NULL,
            stage TEXT NOT NULL DEFAULT 'Proposal',
            currency TEXT NOT NULL DEFAULT 'USD',
            portal_token TEXT NOT NULL DEFAULT '',
            is_deleted INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY(tenant_id) REFERENCES tenants(id)
        )
        "#,
    )
    .execute(&db.0)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS client_contacts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            client_id INTEGER NOT NULL,
            tenant_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            address TEXT NOT NULL,
            email TEXT NOT NULL,
            phone TEXT NOT NULL,
            department TEXT NOT NULL DEFAULT '',
            position TEXT NOT NULL DEFAULT '',
            is_rogue INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY(client_id) REFERENCES clients(id),
            FOREIGN KEY(tenant_id) REFERENCES tenants(id)
        )
        "#,
    )
    .execute(&db.0)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS appointments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tenant_id INTEGER NOT NULL,
            client_id INTEGER NOT NULL,
            contact_id INTEGER NOT NULL,
            title TEXT NOT NULL,
            scheduled_for TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'Scheduled',
            notes TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY(client_id) REFERENCES clients(id),
            FOREIGN KEY(contact_id) REFERENCES client_contacts(id),
            FOREIGN KEY(tenant_id) REFERENCES tenants(id)
        )
        "#,
    )
    .execute(&db.0)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS deployments (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tenant_id INTEGER NOT NULL,
            client_id INTEGER NOT NULL,
            crew_id INTEGER NOT NULL,
            start_at TEXT NOT NULL,
            end_at TEXT NOT NULL,
            fee_per_hour REAL NOT NULL DEFAULT 0,
            info TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'Scheduled',
            deployment_type TEXT NOT NULL DEFAULT 'Onsite',
            required_skills TEXT NOT NULL DEFAULT '',
            compatibility_pref TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY(client_id) REFERENCES clients(id),
            FOREIGN KEY(crew_id) REFERENCES crews(id),
            FOREIGN KEY(tenant_id) REFERENCES tenants(id)
        )
        "#,
    )
    .execute(&db.0)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS deployment_updates (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tenant_id INTEGER NOT NULL,
            deployment_id INTEGER NOT NULL,
            user_id INTEGER,
            work_date TEXT NOT NULL,
            start_time TEXT NOT NULL,
            end_time TEXT NOT NULL,
            hours_worked REAL NOT NULL DEFAULT 0,
            notes TEXT NOT NULL,
            is_placeholder INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(deployment_id, work_date, user_id),
            FOREIGN KEY(deployment_id) REFERENCES deployments(id) ON DELETE CASCADE,
            FOREIGN KEY(tenant_id) REFERENCES tenants(id),
            FOREIGN KEY(user_id) REFERENCES users(id)
        )
        "#,
    )
    .execute(&db.0)
    .await?;
    sqlx::query(
        "UPDATE appointments SET status = 'Attended' WHERE LOWER(status) IN ('on going', 'ongoing')",
    )
    .execute(&db.0)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS invoices (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tenant_id INTEGER NOT NULL,
            deployment_id INTEGER NOT NULL,
            status TEXT NOT NULL DEFAULT 'Draft',
            notes TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(tenant_id, deployment_id),
            FOREIGN KEY(deployment_id) REFERENCES deployments(id) ON DELETE CASCADE,
            FOREIGN KEY(tenant_id) REFERENCES tenants(id)
        )
        "#,
    )
    .execute(&db.0)
    .await?;

    ignore_duplicate_column(
        sqlx::query("ALTER TABLE deployments ADD COLUMN fee_per_hour REAL NOT NULL DEFAULT 0")
            .execute(&db.0)
            .await,
    );

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS outbound_emails (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tenant_id INTEGER NOT NULL,
            client_id INTEGER,
            contact_id INTEGER,
            to_email TEXT NOT NULL,
            cc_emails TEXT NOT NULL DEFAULT '',
            subject TEXT NOT NULL,
            html_body TEXT NOT NULL,
            provider TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'Queued',
            error_message TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY(tenant_id) REFERENCES tenants(id),
            FOREIGN KEY(client_id) REFERENCES clients(id),
            FOREIGN KEY(contact_id) REFERENCES client_contacts(id)
        )
        "#,
    )
    .execute(&db.0)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_permissions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tenant_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
            resource TEXT NOT NULL,
            can_view INTEGER NOT NULL DEFAULT 0,
            can_edit INTEGER NOT NULL DEFAULT 0,
            can_delete INTEGER NOT NULL DEFAULT 0,
            UNIQUE(user_id, resource),
            FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE,
            FOREIGN KEY(tenant_id) REFERENCES tenants(id)
        )
        "#,
    )
    .execute(&db.0)
    .await?;

    ignore_duplicate_column(
        sqlx::query("ALTER TABLE clients ADD COLUMN is_deleted INTEGER NOT NULL DEFAULT 0")
            .execute(&db.0)
            .await,
    );
      ignore_duplicate_column(
          sqlx::query("ALTER TABLE users ADD COLUMN role TEXT NOT NULL DEFAULT 'Owner'")
              .execute(&db.0)
              .await,
      );
      ignore_duplicate_column(
          sqlx::query("ALTER TABLE users ADD COLUMN is_super_admin INTEGER NOT NULL DEFAULT 0")
              .execute(&db.0)
              .await,
      );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE clients ADD COLUMN stage TEXT NOT NULL DEFAULT 'Proposal'")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE clients ADD COLUMN currency TEXT NOT NULL DEFAULT 'USD'")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE clients ADD COLUMN portal_token TEXT NOT NULL DEFAULT ''")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE tenants ADD COLUMN email_provider TEXT NOT NULL DEFAULT 'Mailtrap'")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE tenants ADD COLUMN email_from_name TEXT NOT NULL DEFAULT ''")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE tenants ADD COLUMN email_from_address TEXT NOT NULL DEFAULT ''")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE tenants ADD COLUMN smtp_host TEXT NOT NULL DEFAULT ''")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE tenants ADD COLUMN smtp_port TEXT NOT NULL DEFAULT ''")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE tenants ADD COLUMN smtp_username TEXT NOT NULL DEFAULT ''")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE tenants ADD COLUMN smtp_password TEXT NOT NULL DEFAULT ''")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE tenants ADD COLUMN smtp_encryption TEXT NOT NULL DEFAULT ''")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE tenants ADD COLUMN mailgun_domain TEXT NOT NULL DEFAULT ''")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE tenants ADD COLUMN mailgun_api_key TEXT NOT NULL DEFAULT ''")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE tenants ADD COLUMN postmark_server_token TEXT NOT NULL DEFAULT ''")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE tenants ADD COLUMN resend_api_key TEXT NOT NULL DEFAULT ''")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE tenants ADD COLUMN ses_access_key TEXT NOT NULL DEFAULT ''")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE tenants ADD COLUMN ses_secret_key TEXT NOT NULL DEFAULT ''")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE tenants ADD COLUMN ses_region TEXT NOT NULL DEFAULT ''")
            .execute(&db.0)
            .await,
    );
      ignore_duplicate_column(
          sqlx::query("ALTER TABLE tenants ADD COLUMN sendmail_path TEXT NOT NULL DEFAULT ''")
              .execute(&db.0)
              .await,
      );
      ignore_duplicate_column(
          sqlx::query("ALTER TABLE tenants ADD COLUMN app_name TEXT NOT NULL DEFAULT 'Kinetic'")
              .execute(&db.0)
              .await,
      );
      ignore_duplicate_column(
          sqlx::query("ALTER TABLE tenants ADD COLUMN logo_path TEXT NOT NULL DEFAULT ''")
              .execute(&db.0)
              .await,
      );
      ignore_duplicate_column(
          sqlx::query("ALTER TABLE tenants ADD COLUMN theme_key TEXT NOT NULL DEFAULT 'kinetic'")
              .execute(&db.0)
              .await,
      );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE tenants ADD COLUMN background_hue INTEGER NOT NULL DEFAULT 32")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query(
            "ALTER TABLE tenants ADD COLUMN body_font TEXT NOT NULL DEFAULT 'Space Grotesk'",
        )
        .execute(&db.0)
        .await,
    );
    ignore_duplicate_column(
        sqlx::query(
            "ALTER TABLE tenants ADD COLUMN heading_font TEXT NOT NULL DEFAULT 'Space Grotesk'",
        )
        .execute(&db.0)
        .await,
    );
      ignore_duplicate_column(
          sqlx::query("ALTER TABLE tenants ADD COLUMN plan_key TEXT NOT NULL DEFAULT 'free'")
              .execute(&db.0)
              .await,
      );
    ignore_duplicate_column(
        sqlx::query(
            "ALTER TABLE tenants ADD COLUMN plan_started_at TEXT NOT NULL DEFAULT '1970-01-01 00:00:00'",
        )
        .execute(&db.0)
        .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE tenants ADD COLUMN plan_expired INTEGER NOT NULL DEFAULT 0")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE client_contacts ADD COLUMN is_rogue INTEGER NOT NULL DEFAULT 0")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE client_contacts ADD COLUMN department TEXT NOT NULL DEFAULT ''")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE client_contacts ADD COLUMN position TEXT NOT NULL DEFAULT ''")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE crew_members ADD COLUMN user_id INTEGER")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query(
            "ALTER TABLE crews ADD COLUMN gear_score INTEGER NOT NULL DEFAULT 100",
        )
        .execute(&db.0)
        .await,
    );
    ignore_duplicate_column(
        sqlx::query(
            "ALTER TABLE crew_members ADD COLUMN availability_status TEXT NOT NULL DEFAULT 'Available'",
        )
        .execute(&db.0)
        .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE crews ADD COLUMN skill_tags TEXT NOT NULL DEFAULT ''")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE crews ADD COLUMN compatibility_tags TEXT NOT NULL DEFAULT ''")
            .execute(&db.0)
            .await,
    );
    ensure_deployment_updates_user_id(db).await?;
    ignore_duplicate_column(
        sqlx::query(
            "ALTER TABLE deployment_updates ADD COLUMN is_placeholder INTEGER NOT NULL DEFAULT 0",
        )
        .execute(&db.0)
        .await,
    );
    ignore_duplicate_column(
        sqlx::query(
            "ALTER TABLE deployments ADD COLUMN deployment_type TEXT NOT NULL DEFAULT 'Onsite'",
        )
        .execute(&db.0)
        .await,
    );
    ignore_duplicate_column(
        sqlx::query("ALTER TABLE deployments ADD COLUMN required_skills TEXT NOT NULL DEFAULT ''")
            .execute(&db.0)
            .await,
    );
    ignore_duplicate_column(
        sqlx::query(
            "ALTER TABLE deployments ADD COLUMN compatibility_pref TEXT NOT NULL DEFAULT ''",
        )
        .execute(&db.0)
        .await,
    );
    ensure_deployment_updates_unique_per_user(db).await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS work_timers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tenant_id INTEGER NOT NULL,
            deployment_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
            start_at TEXT NOT NULL,
            end_at TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY(tenant_id) REFERENCES tenants(id),
            FOREIGN KEY(deployment_id) REFERENCES deployments(id),
            FOREIGN KEY(user_id) REFERENCES users(id)
        )
        "#,
    )
    .execute(&db.0)
    .await?;
    ignore_duplicate_column(
        sqlx::query(
            "ALTER TABLE outbound_emails ADD COLUMN error_message TEXT NOT NULL DEFAULT ''",
        )
        .execute(&db.0)
        .await,
    );

    sqlx::query(
        "UPDATE tenants SET plan_started_at = datetime('now') WHERE plan_started_at = '1970-01-01 00:00:00' OR plan_started_at = ''",
    )
    .execute(&db.0)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS plan_limits (
            plan_key TEXT PRIMARY KEY,
            clients INTEGER NOT NULL,
            contacts_per_client INTEGER NOT NULL,
            appointments_per_client INTEGER NOT NULL,
            deployments_per_client INTEGER NOT NULL,
            crews INTEGER NOT NULL,
            members_per_crew INTEGER NOT NULL,
            users INTEGER NOT NULL,
            expires_after_days INTEGER NOT NULL DEFAULT 30
        )
        "#,
    )
    .execute(&db.0)
    .await?;
    ignore_duplicate_column(
        sqlx::query(
            "ALTER TABLE plan_limits ADD COLUMN expires_after_days INTEGER NOT NULL DEFAULT 30",
        )
        .execute(&db.0)
        .await,
    );

    sqlx::query(
        r#"
        INSERT OR IGNORE INTO plan_limits
            (plan_key, clients, contacts_per_client, appointments_per_client, deployments_per_client, crews, members_per_crew, users, expires_after_days)
        VALUES
            ('free', 5, 5, 20, 1, 2, 5, 11, 30),
            ('pro', 20, 5, 40, 5, 5, 10, 51, 180),
            ('enterprise', 0, 0, 0, 0, 0, 0, 0, 365)
        "#,
    )
    .execute(&db.0)
    .await?;

    seed_admin(db).await?;
    seed_client_data(db).await?;

    Ok(())
}

fn ignore_duplicate_column(result: Result<sqlx::sqlite::SqliteQueryResult, sqlx::Error>) {
    if let Err(err) = result {
        let message = err.to_string();
        if !message.contains("duplicate column") {
            eprintln!("Schema update error: {message}");
        }
    }
}

async fn ensure_deployment_updates_user_id(db: &Db) -> Result<(), sqlx::Error> {
    let columns = sqlx::query("PRAGMA table_info(deployment_updates)")
        .fetch_all(&db.0)
        .await?;
    let has_user_id = columns
        .iter()
        .any(|row| row.get::<String, _>("name") == "user_id");
    if !has_user_id {
        sqlx::query("ALTER TABLE deployment_updates ADD COLUMN user_id INTEGER")
            .execute(&db.0)
            .await?;
        sqlx::query(
            r#"
            UPDATE deployment_updates
            SET user_id = (
                SELECT cm.user_id
                FROM deployments d
                JOIN crew_members cm ON cm.crew_id = d.crew_id
                WHERE d.id = deployment_updates.deployment_id
                  AND cm.user_id IS NOT NULL
                ORDER BY cm.id
                LIMIT 1
            )
            WHERE user_id IS NULL
            "#,
        )
        .execute(&db.0)
        .await?;
    }
    Ok(())
}

async fn ensure_deployment_updates_unique_per_user(db: &Db) -> Result<(), sqlx::Error> {
    let index_rows = sqlx::query("PRAGMA index_list('deployment_updates')")
        .fetch_all(&db.0)
        .await?;
    let mut needs_rebuild = false;
    for row in index_rows {
        let is_unique = row.get::<i64, _>("unique") != 0;
        if !is_unique {
            continue;
        }
        let index_name: String = row.get("name");
        let columns = sqlx::query(&format!("PRAGMA index_info('{index_name}')"))
            .fetch_all(&db.0)
            .await?;
        let names = columns
            .into_iter()
            .map(|col| col.get::<String, _>("name"))
            .collect::<Vec<_>>();
        if names == vec!["deployment_id", "work_date"] {
            needs_rebuild = true;
            break;
        }
    }
    if !needs_rebuild {
        return Ok(());
    }

    sqlx::query("ALTER TABLE deployment_updates RENAME TO deployment_updates_old")
        .execute(&db.0)
        .await?;
    sqlx::query(
        r#"
        CREATE TABLE deployment_updates (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            tenant_id INTEGER NOT NULL,
            deployment_id INTEGER NOT NULL,
            user_id INTEGER,
            work_date TEXT NOT NULL,
            start_time TEXT NOT NULL,
            end_time TEXT NOT NULL,
            hours_worked REAL NOT NULL DEFAULT 0,
            notes TEXT NOT NULL,
            is_placeholder INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            UNIQUE(deployment_id, work_date, user_id),
            FOREIGN KEY(deployment_id) REFERENCES deployments(id) ON DELETE CASCADE,
            FOREIGN KEY(tenant_id) REFERENCES tenants(id),
            FOREIGN KEY(user_id) REFERENCES users(id)
        )
        "#,
    )
    .execute(&db.0)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO deployment_updates
            (id, tenant_id, deployment_id, user_id, work_date, start_time, end_time, hours_worked, notes, is_placeholder, created_at)
        SELECT id, tenant_id, deployment_id, user_id, work_date, start_time, end_time, hours_worked, notes, is_placeholder, created_at
        FROM deployment_updates_old
        "#,
    )
    .execute(&db.0)
    .await?;
    sqlx::query("DROP TABLE deployment_updates_old")
        .execute(&db.0)
        .await?;
    Ok(())
}

async fn seed_admin(db: &Db) -> Result<(), sqlx::Error> {
    let admin_slug = "admin";
    let admin_name = "Admin workspace";
    let admin_email = "admin@kinetic.app";
    let admin_password = "Angelus69@@@";

    let tenant_id = match tenant_repo::find_tenant_id_by_slug(db, admin_slug).await? {
        Some(id) => id,
        None => tenant_repo::create_tenant(db, admin_slug, admin_name, "enterprise").await?,
    };

    let existing = user_repo::find_super_admin_auth_by_email(db, admin_email).await?;
    if existing.is_some() {
        return Ok(());
    }

    let hash = hash_password(admin_password).map_err(|_| sqlx::Error::RowNotFound)?;
    user_repo::create_super_admin(db, tenant_id, &admin_email.to_lowercase(), &hash).await?;

    Ok(())
}

async fn seed_client_data(db: &Db) -> Result<(), sqlx::Error> {
    let tenant_row = sqlx::query("SELECT id FROM tenants ORDER BY id ASC LIMIT 1")
        .fetch_optional(&db.0)
        .await?;
    let tenant_id: i64 = match tenant_row {
        Some(row) => row.get("id"),
        None => return Ok(()),
    };

    let client_count: i64 = sqlx::query("SELECT COUNT(*) as count FROM clients WHERE tenant_id = ?")
        .bind(tenant_id)
        .fetch_one(&db.0)
        .await?
        .get("count");
    let contact_count: i64 =
        sqlx::query("SELECT COUNT(*) as count FROM client_contacts WHERE tenant_id = ?")
            .bind(tenant_id)
            .fetch_one(&db.0)
            .await?
            .get("count");
    let appointment_count: i64 =
        sqlx::query("SELECT COUNT(*) as count FROM appointments WHERE tenant_id = ?")
            .bind(tenant_id)
            .fetch_one(&db.0)
            .await?
            .get("count");

    if client_count > 0 || contact_count > 0 || appointment_count > 0 {
        return Ok(());
    }

    let mut tx = db.0.begin().await?;
    let mut client_ids = Vec::new();
    let mut contact_ids = Vec::new();
    let stages = ["Proposal", "Negotiation", "Closed"];

    for index in 1..=50 {
        let stage = stages[(index - 1) % stages.len()];
        sqlx::query(
            r#"
            INSERT INTO clients
                (tenant_id, company_name, address, phone, email, latitude, longitude, stage, currency)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(tenant_id)
        .bind(format!("Client {index}"))
        .bind(format!("{index} Kinetic Way"))
        .bind(format!("555-010{:02}", index % 100))
        .bind(format!("client{index}@example.com"))
        .bind("37.7749")
        .bind("-122.4194")
        .bind(stage)
        .bind("USD")
        .execute(&mut *tx)
        .await?;
        let client_id: i64 = sqlx::query("SELECT last_insert_rowid() as id")
            .fetch_one(&mut *tx)
            .await?
            .get("id");
        client_ids.push(client_id);
    }

    for index in 1..=50 {
        let client_id = client_ids[(index - 1) % client_ids.len()];
        sqlx::query(
            r#"
            INSERT INTO client_contacts
                (client_id, tenant_id, name, address, email, phone, department, position)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(client_id)
        .bind(tenant_id)
        .bind(format!("Contact {index}"))
        .bind(format!("{index} Kinetic Way"))
        .bind(format!("contact{index}@example.com"))
        .bind(format!("555-020{:02}", index % 100))
        .bind(format!("Department {}", (index - 1) % 5 + 1))
        .bind(format!("Position {}", (index - 1) % 4 + 1))
        .execute(&mut *tx)
        .await?;
        let contact_id: i64 = sqlx::query("SELECT last_insert_rowid() as id")
            .fetch_one(&mut *tx)
            .await?
            .get("id");
        contact_ids.push(contact_id);
    }

    for index in 1..=50 {
        let contact_id = contact_ids[(index - 1) % contact_ids.len()];
        let client_id = client_ids[(index - 1) % client_ids.len()];
        let scheduled_for = format!("2026-01-{:02} 09:{:02}", (index - 1) % 28 + 1, index % 60);
        let status = match index % 5 {
            0 => "No-Show",
            1 => "Scheduled",
            2 => "Confirmed",
            3 => "Attended",
            _ => "Cancelled",
        };
        sqlx::query(
            r#"
            INSERT INTO appointments
                (tenant_id, client_id, contact_id, title, scheduled_for, status, notes)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(tenant_id)
        .bind(client_id)
        .bind(contact_id)
        .bind(format!("Meeting {index}"))
        .bind(scheduled_for)
        .bind(status)
        .bind(format!("Notes for meeting {index}."))
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(())
}
