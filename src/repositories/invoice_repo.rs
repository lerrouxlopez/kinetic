use rocket_db_pools::sqlx::{self, Row};

use crate::models::{Invoice, InvoiceCandidate, InvoiceSummary};
use crate::Db;

pub async fn list_invoices_with_details(
    db: &Db,
    tenant_id: i64,
) -> Result<Vec<InvoiceSummary>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            invoices.id as invoice_id,
            invoices.deployment_id as deployment_id,
            invoices.status as status,
            invoices.notes as notes,
            invoices.created_at as created_at,
            clients.id as client_id,
            clients.company_name as client_name,
            clients.address as client_address,
            clients.email as client_email,
            clients.currency as client_currency,
            crews.id as crew_id,
            crews.name as crew_name,
            deployments.start_at as start_at,
            deployments.end_at as end_at,
            deployments.fee_per_hour as fee_per_hour,
            COALESCE(SUM(deployment_updates.hours_worked), 0.0) as total_hours
        FROM invoices
        JOIN deployments ON invoices.deployment_id = deployments.id
        JOIN clients ON deployments.client_id = clients.id
        JOIN crews ON deployments.crew_id = crews.id
        LEFT JOIN deployment_updates
            ON deployment_updates.deployment_id = deployments.id
            AND deployment_updates.tenant_id = invoices.tenant_id
        WHERE invoices.tenant_id = ?
        GROUP BY invoices.id
        ORDER BY invoices.created_at DESC, invoices.id DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| InvoiceSummary {
            id: row.get("invoice_id"),
            deployment_id: row.get("deployment_id"),
            status: row.get("status"),
            notes: row.get("notes"),
            created_at: row.get("created_at"),
            client_id: row.get("client_id"),
            client_name: row.get("client_name"),
            client_address: row.get("client_address"),
            client_email: row.get("client_email"),
            client_currency: row.get("client_currency"),
            crew_id: row.get("crew_id"),
            crew_name: row.get("crew_name"),
            start_at: row.get("start_at"),
            end_at: row.get("end_at"),
            fee_per_hour: row.get("fee_per_hour"),
            total_hours: row.get("total_hours"),
        })
        .collect())
}

pub async fn find_invoice_with_details(
    db: &Db,
    tenant_id: i64,
    invoice_id: i64,
) -> Result<Option<InvoiceSummary>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT
            invoices.id as invoice_id,
            invoices.deployment_id as deployment_id,
            invoices.status as status,
            invoices.notes as notes,
            invoices.created_at as created_at,
            clients.id as client_id,
            clients.company_name as client_name,
            clients.address as client_address,
            clients.email as client_email,
            clients.currency as client_currency,
            crews.id as crew_id,
            crews.name as crew_name,
            deployments.start_at as start_at,
            deployments.end_at as end_at,
            deployments.fee_per_hour as fee_per_hour,
            COALESCE(SUM(deployment_updates.hours_worked), 0.0) as total_hours
        FROM invoices
        JOIN deployments ON invoices.deployment_id = deployments.id
        JOIN clients ON deployments.client_id = clients.id
        JOIN crews ON deployments.crew_id = crews.id
        LEFT JOIN deployment_updates
            ON deployment_updates.deployment_id = deployments.id
            AND deployment_updates.tenant_id = invoices.tenant_id
        WHERE invoices.tenant_id = ? AND invoices.id = ?
        GROUP BY invoices.id
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(invoice_id)
    .fetch_optional(&db.0)
    .await?;

    Ok(row.map(|row| InvoiceSummary {
        id: row.get("invoice_id"),
        deployment_id: row.get("deployment_id"),
        status: row.get("status"),
        notes: row.get("notes"),
        created_at: row.get("created_at"),
        client_id: row.get("client_id"),
        client_name: row.get("client_name"),
        client_address: row.get("client_address"),
        client_email: row.get("client_email"),
        client_currency: row.get("client_currency"),
        crew_id: row.get("crew_id"),
        crew_name: row.get("crew_name"),
        start_at: row.get("start_at"),
        end_at: row.get("end_at"),
        fee_per_hour: row.get("fee_per_hour"),
        total_hours: row.get("total_hours"),
    }))
}

pub async fn find_invoice_by_id(
    db: &Db,
    tenant_id: i64,
    invoice_id: i64,
) -> Result<Option<Invoice>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, tenant_id, deployment_id, status, notes, created_at FROM invoices WHERE id = ? AND tenant_id = ?",
    )
    .bind(invoice_id)
    .bind(tenant_id)
    .fetch_optional(&db.0)
    .await?;

    Ok(row.map(|row| Invoice {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        deployment_id: row.get("deployment_id"),
        status: row.get("status"),
        notes: row.get("notes"),
        created_at: row.get("created_at"),
    }))
}

pub async fn find_invoice_by_deployment(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
) -> Result<Option<Invoice>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, tenant_id, deployment_id, status, notes, created_at FROM invoices WHERE deployment_id = ? AND tenant_id = ?",
    )
    .bind(deployment_id)
    .bind(tenant_id)
    .fetch_optional(&db.0)
    .await?;

    Ok(row.map(|row| Invoice {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        deployment_id: row.get("deployment_id"),
        status: row.get("status"),
        notes: row.get("notes"),
        created_at: row.get("created_at"),
    }))
}

pub async fn create_invoice(
    db: &Db,
    tenant_id: i64,
    deployment_id: i64,
    status: &str,
    notes: &str,
) -> Result<i64, sqlx::Error> {
    let result = sqlx::query(
        "INSERT INTO invoices (tenant_id, deployment_id, status, notes) VALUES (?, ?, ?, ?)",
    )
    .bind(tenant_id)
    .bind(deployment_id)
    .bind(status)
    .bind(notes)
    .execute(&db.0)
    .await?;

    Ok(result.last_insert_rowid())
}

pub async fn update_invoice(
    db: &Db,
    tenant_id: i64,
    invoice_id: i64,
    status: &str,
    notes: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE invoices SET status = ?, notes = ? WHERE id = ? AND tenant_id = ?")
        .bind(status)
        .bind(notes)
        .bind(invoice_id)
        .bind(tenant_id)
        .execute(&db.0)
        .await?;
    Ok(())
}

pub async fn delete_invoice(
    db: &Db,
    tenant_id: i64,
    invoice_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM invoices WHERE id = ? AND tenant_id = ?")
        .bind(invoice_id)
        .bind(tenant_id)
        .execute(&db.0)
        .await?;
    Ok(())
}

pub async fn list_completed_deployments_without_invoice(
    db: &Db,
    tenant_id: i64,
) -> Result<Vec<InvoiceCandidate>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT
            deployments.id as deployment_id,
            clients.company_name as client_name,
            crews.name as crew_name,
            deployments.start_at as start_at,
            deployments.end_at as end_at,
            deployments.fee_per_hour as fee_per_hour,
            clients.currency as client_currency,
            COALESCE(SUM(deployment_updates.hours_worked), 0.0) as total_hours
        FROM deployments
        JOIN clients ON deployments.client_id = clients.id
        JOIN crews ON deployments.crew_id = crews.id
        LEFT JOIN invoices
            ON invoices.deployment_id = deployments.id
            AND invoices.tenant_id = deployments.tenant_id
        LEFT JOIN deployment_updates
            ON deployment_updates.deployment_id = deployments.id
            AND deployment_updates.tenant_id = deployments.tenant_id
        WHERE deployments.tenant_id = ?
            AND deployments.status = 'Completed'
            AND invoices.id IS NULL
        GROUP BY deployments.id
        ORDER BY deployments.end_at DESC, deployments.id DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| InvoiceCandidate {
            deployment_id: row.get("deployment_id"),
            client_name: row.get("client_name"),
            crew_name: row.get("crew_name"),
            start_at: row.get("start_at"),
            end_at: row.get("end_at"),
            fee_per_hour: row.get("fee_per_hour"),
            client_currency: row.get("client_currency"),
            total_hours: row.get("total_hours"),
        })
        .collect())
}

pub async fn count_invoices(db: &Db, tenant_id: i64) -> Result<i64, sqlx::Error> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM invoices WHERE tenant_id = ?")
        .bind(tenant_id)
        .fetch_one(&db.0)
        .await?;
    Ok(row.get("count"))
}

pub async fn count_invoices_by_status(
    db: &Db,
    tenant_id: i64,
) -> Result<Vec<(String, i64)>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT status, COUNT(*) as count FROM invoices WHERE tenant_id = ? GROUP BY status",
    )
    .bind(tenant_id)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| (row.get("status"), row.get("count")))
        .collect())
}
