use rocket_db_pools::sqlx::{self, Row};

use crate::models::{Client, ClientContact};
use crate::Db;

pub async fn list_clients(db: &Db, tenant_id: i64) -> Result<Vec<Client>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, tenant_id, company_name, address, phone, email, latitude, longitude, stage, currency
        FROM clients
        WHERE tenant_id = ? AND is_deleted = 0
        ORDER BY id DESC
        "#,
    )
    .bind(tenant_id)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| Client {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            company_name: row.get("company_name"),
            address: row.get("address"),
            phone: row.get("phone"),
            email: row.get("email"),
            latitude: row.get("latitude"),
            longitude: row.get("longitude"),
            stage: row.get("stage"),
            currency: row.get("currency"),
        })
        .collect())
}

pub async fn list_clients_paged(
    db: &Db,
    tenant_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<Client>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, tenant_id, company_name, address, phone, email, latitude, longitude, stage, currency
        FROM clients
        WHERE tenant_id = ? AND is_deleted = 0
        ORDER BY id DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(tenant_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| Client {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            company_name: row.get("company_name"),
            address: row.get("address"),
            phone: row.get("phone"),
            email: row.get("email"),
            latitude: row.get("latitude"),
            longitude: row.get("longitude"),
            stage: row.get("stage"),
            currency: row.get("currency"),
        })
        .collect())
}

pub async fn count_clients(db: &Db, tenant_id: i64) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        "SELECT COUNT(*) as count FROM clients WHERE tenant_id = ? AND is_deleted = 0",
    )
    .bind(tenant_id)
    .fetch_one(&db.0)
    .await?;
    Ok(row.get("count"))
}

pub async fn find_client_by_id(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
) -> Result<Option<Client>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id, tenant_id, company_name, address, phone, email, latitude, longitude, stage, currency
        FROM clients
        WHERE id = ? AND tenant_id = ? AND is_deleted = 0
        "#,
    )
    .bind(client_id)
    .bind(tenant_id)
    .fetch_optional(&db.0)
    .await?;

    Ok(row.map(|row| Client {
        id: row.get("id"),
        tenant_id: row.get("tenant_id"),
        company_name: row.get("company_name"),
        address: row.get("address"),
        phone: row.get("phone"),
        email: row.get("email"),
        latitude: row.get("latitude"),
        longitude: row.get("longitude"),
        stage: row.get("stage"),
        currency: row.get("currency"),
    }))
}

pub async fn create_client(
    db: &Db,
    tenant_id: i64,
    company_name: &str,
    address: &str,
    phone: &str,
    email: &str,
    latitude: &str,
    longitude: &str,
    stage: &str,
    currency: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO clients (tenant_id, company_name, address, phone, email, latitude, longitude, stage, currency)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(tenant_id)
    .bind(company_name)
    .bind(address)
    .bind(phone)
    .bind(email)
    .bind(latitude)
    .bind(longitude)
    .bind(stage)
    .bind(currency)
    .execute(&db.0)
    .await?;
    Ok(())
}

pub async fn update_client(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
    company_name: &str,
    address: &str,
    phone: &str,
    email: &str,
    latitude: &str,
    longitude: &str,
    stage: &str,
    currency: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE clients
        SET company_name = ?, address = ?, phone = ?, email = ?, latitude = ?, longitude = ?, stage = ?, currency = ?
        WHERE id = ? AND tenant_id = ?
        "#,
    )
    .bind(company_name)
    .bind(address)
    .bind(phone)
    .bind(email)
    .bind(latitude)
    .bind(longitude)
    .bind(stage)
    .bind(currency)
    .bind(client_id)
    .bind(tenant_id)
    .execute(&db.0)
    .await?;
    Ok(())
}

pub async fn mark_client_deleted(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE clients SET is_deleted = 1 WHERE id = ? AND tenant_id = ?")
        .bind(client_id)
        .bind(tenant_id)
        .execute(&db.0)
        .await?;
    Ok(())
}

pub async fn list_contacts(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
) -> Result<Vec<ClientContact>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, client_id, tenant_id, name, address, email, phone, department, position
        FROM client_contacts
        WHERE client_id = ? AND tenant_id = ? AND is_rogue = 0
        ORDER BY id DESC
        "#,
    )
    .bind(client_id)
    .bind(tenant_id)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| ClientContact {
            id: row.get("id"),
            client_id: row.get("client_id"),
            tenant_id: row.get("tenant_id"),
            name: row.get("name"),
            address: row.get("address"),
            email: row.get("email"),
            phone: row.get("phone"),
            department: row.get("department"),
            position: row.get("position"),
        })
        .collect())
}

pub async fn list_contacts_paged(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<ClientContact>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, client_id, tenant_id, name, address, email, phone, department, position
        FROM client_contacts
        WHERE client_id = ? AND tenant_id = ? AND is_rogue = 0
        ORDER BY id DESC
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(client_id)
    .bind(tenant_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&db.0)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| ClientContact {
            id: row.get("id"),
            client_id: row.get("client_id"),
            tenant_id: row.get("tenant_id"),
            name: row.get("name"),
            address: row.get("address"),
            email: row.get("email"),
            phone: row.get("phone"),
            department: row.get("department"),
            position: row.get("position"),
        })
        .collect())
}

pub async fn count_contacts(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query(
        "SELECT COUNT(*) as count FROM client_contacts WHERE client_id = ? AND tenant_id = ? AND is_rogue = 0",
    )
    .bind(client_id)
    .bind(tenant_id)
    .fetch_one(&db.0)
    .await?;
    Ok(row.get("count"))
}

pub async fn find_contact_by_id(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
    contact_id: i64,
) -> Result<Option<ClientContact>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id, client_id, tenant_id, name, address, email, phone, department, position
        FROM client_contacts
        WHERE id = ? AND client_id = ? AND tenant_id = ? AND is_rogue = 0
        "#,
    )
    .bind(contact_id)
    .bind(client_id)
    .bind(tenant_id)
    .fetch_optional(&db.0)
    .await?;

    Ok(row.map(|row| ClientContact {
        id: row.get("id"),
        client_id: row.get("client_id"),
        tenant_id: row.get("tenant_id"),
        name: row.get("name"),
        address: row.get("address"),
        email: row.get("email"),
        phone: row.get("phone"),
        department: row.get("department"),
        position: row.get("position"),
    }))
}

pub async fn create_contact(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
    name: &str,
    address: &str,
    email: &str,
    phone: &str,
    department: &str,
    position: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO client_contacts (client_id, tenant_id, name, address, email, phone, department, position)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(client_id)
    .bind(tenant_id)
    .bind(name)
    .bind(address)
    .bind(email)
    .bind(phone)
    .bind(department)
    .bind(position)
    .execute(&db.0)
    .await?;
    Ok(())
}

pub async fn update_contact(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
    contact_id: i64,
    name: &str,
    address: &str,
    email: &str,
    phone: &str,
    department: &str,
    position: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE client_contacts
        SET name = ?, address = ?, email = ?, phone = ?, department = ?, position = ?
        WHERE id = ? AND client_id = ? AND tenant_id = ?
        "#,
    )
    .bind(name)
    .bind(address)
    .bind(email)
    .bind(phone)
    .bind(department)
    .bind(position)
    .bind(contact_id)
    .bind(client_id)
    .bind(tenant_id)
    .execute(&db.0)
    .await?;
    Ok(())
}

pub async fn delete_contact(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
    contact_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM client_contacts WHERE id = ? AND client_id = ? AND tenant_id = ?",
    )
    .bind(contact_id)
    .bind(client_id)
    .bind(tenant_id)
    .execute(&db.0)
    .await?;
    Ok(())
}

pub async fn delete_contacts_by_client(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE client_contacts SET is_rogue = 1 WHERE client_id = ? AND tenant_id = ?")
        .bind(client_id)
        .bind(tenant_id)
        .execute(&db.0)
        .await?;
    Ok(())
}
