use rocket_db_pools::sqlx::{self, Row};

use crate::models::Appointment;
use crate::Db;

pub async fn list_appointments_by_client(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
) -> Result<Vec<Appointment>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT appointments.id,
               appointments.client_id,
               appointments.contact_id,
               appointments.tenant_id,
               appointments.title,
               appointments.scheduled_for,
               appointments.status,
               appointments.notes,
               client_contacts.name as contact_name
        FROM appointments
        JOIN client_contacts ON client_contacts.id = appointments.contact_id
        WHERE appointments.tenant_id = ? AND appointments.client_id = ?
        ORDER BY appointments.scheduled_for DESC, appointments.id DESC
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| Appointment {
            id: row.get("id"),
            client_id: row.get("client_id"),
            contact_id: row.get("contact_id"),
            tenant_id: row.get("tenant_id"),
            contact_name: row.get("contact_name"),
            title: row.get("title"),
            scheduled_for: row.get("scheduled_for"),
            status: row.get("status"),
            notes: row.get("notes"),
        })
        .collect())
}

pub async fn list_appointments_by_client_paged(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<Appointment>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT appointments.id,
               appointments.client_id,
               appointments.contact_id,
               appointments.tenant_id,
               appointments.title,
               appointments.scheduled_for,
               appointments.status,
               appointments.notes,
               client_contacts.name as contact_name
        FROM appointments
        JOIN client_contacts ON client_contacts.id = appointments.contact_id
        WHERE appointments.tenant_id = ? AND appointments.client_id = ?
        ORDER BY appointments.scheduled_for DESC, appointments.id DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| Appointment {
            id: row.get("id"),
            client_id: row.get("client_id"),
            contact_id: row.get("contact_id"),
            tenant_id: row.get("tenant_id"),
            contact_name: row.get("contact_name"),
            title: row.get("title"),
            scheduled_for: row.get("scheduled_for"),
            status: row.get("status"),
            notes: row.get("notes"),
        })
        .collect())
}

pub async fn count_appointments_by_client(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        "SELECT COUNT(*) as count FROM appointments WHERE tenant_id = ? AND client_id = ?",
    )
    .bind(tenant_id)
    .bind(client_id)
    .fetch_one(&db.0)
    .await?;
    Ok(row.get("count"))
}

pub async fn count_appointments(
    db: &Db,
    tenant_id: i64,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM appointments WHERE tenant_id = ?")
        .bind(tenant_id)
        .fetch_one(&db.0)
        .await?;
    Ok(row.get("count"))
}

pub async fn count_appointments_total(db: &Db) -> Result<i64, sqlx::Error> {
    let row = sqlx::query("SELECT COUNT(*) as count FROM appointments")
        .fetch_one(&db.0)
        .await?;
    Ok(row.get("count"))
}

pub async fn count_appointments_by_status(
    db: &Db,
    tenant_id: i64,
) -> Result<Vec<(String, i64)>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT status, COUNT(*) as count FROM appointments WHERE tenant_id = ? GROUP BY status",
    )
    .bind(tenant_id)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| (row.get("status"), row.get("count")))
        .collect())
}

pub async fn find_appointment_by_id(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
    appointment_id: i64,
) -> Result<Option<Appointment>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT appointments.id,
               appointments.client_id,
               appointments.contact_id,
               appointments.tenant_id,
               appointments.title,
               appointments.scheduled_for,
               appointments.status,
               appointments.notes,
               client_contacts.name as contact_name
        FROM appointments
        JOIN client_contacts ON client_contacts.id = appointments.contact_id
        WHERE appointments.tenant_id = ? AND appointments.client_id = ? AND appointments.id = ?
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .bind(appointment_id)
    .fetch_optional(&db.0)
    .await?;

    Ok(row.map(|row| Appointment {
        id: row.get("id"),
        client_id: row.get("client_id"),
        contact_id: row.get("contact_id"),
        tenant_id: row.get("tenant_id"),
        contact_name: row.get("contact_name"),
        title: row.get("title"),
        scheduled_for: row.get("scheduled_for"),
        status: row.get("status"),
        notes: row.get("notes"),
    }))
}

pub async fn create_appointment(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
    contact_id: i64,
    title: &str,
    scheduled_for: &str,
    status: &str,
    notes: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO appointments (
            tenant_id,
            client_id,
            contact_id,
            title,
            scheduled_for,
            status,
            notes
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(tenant_id)
    .bind(client_id)
    .bind(contact_id)
    .bind(title)
    .bind(scheduled_for)
    .bind(status)
    .bind(notes)
    .execute(&db.0)
    .await?;
    Ok(())
}

pub async fn update_appointment(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
    appointment_id: i64,
    title: &str,
    scheduled_for: &str,
    status: &str,
    notes: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE appointments
        SET title = ?, scheduled_for = ?, status = ?, notes = ?
        WHERE tenant_id = ? AND client_id = ? AND id = ?
        "#,
    )
    .bind(title)
    .bind(scheduled_for)
    .bind(status)
    .bind(notes)
    .bind(tenant_id)
    .bind(client_id)
    .bind(appointment_id)
    .execute(&db.0)
    .await?;
    Ok(())
}

pub async fn delete_appointment(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
    appointment_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM appointments WHERE tenant_id = ? AND client_id = ? AND id = ?")
        .bind(tenant_id)
        .bind(client_id)
        .bind(appointment_id)
        .execute(&db.0)
        .await?;
    Ok(())
}
