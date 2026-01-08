use rocket_db_pools::sqlx;

use crate::models::{
    Client,
    ClientContact,
    ClientContactForm,
    ClientContactFormView,
    ClientForm,
    ClientFormView,
};
use crate::repositories::client_repo;
use crate::services::workspace_service;
use crate::Db;

pub struct ClientError {
    pub message: String,
    pub form: ClientFormView,
}

pub struct ContactError {
    pub message: String,
    pub form: ClientContactFormView,
}

pub fn client_stage_options() -> Vec<&'static str> {
    vec!["Proposal", "Negotiation", "Closed"]
}

pub fn currency_options() -> Vec<&'static str> {
    vec![
        "USD", "EUR", "GBP", "CAD", "AUD", "NZD", "JPY", "CNY", "HKD", "SGD",
        "INR", "KRW", "BRL", "MXN", "ARS", "CLP", "COP", "PEN", "ZAR", "NGN",
        "EGP", "MAD", "KES", "GHS", "TZS", "UGX", "RWF", "ILS", "AED", "SAR",
        "QAR", "KWD", "BHD", "OMR", "TRY", "PLN", "CZK", "HUF", "RON", "SEK",
        "NOK", "DKK", "CHF", "RUB", "UAH", "BGN", "ISK", "PKR", "BDT", "LKR",
        "THB", "MYR", "IDR", "VND", "PHP",
    ]
}

pub async fn list_clients(db: &Db, tenant_id: i64) -> Result<Vec<Client>, sqlx::Error> {
    client_repo::list_clients(db, tenant_id).await
}

pub async fn list_clients_paged(
    db: &Db,
    tenant_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<Client>, sqlx::Error> {
    client_repo::list_clients_paged(db, tenant_id, limit, offset).await
}

pub async fn count_clients(db: &Db, tenant_id: i64) -> Result<i64, sqlx::Error> {
    client_repo::count_clients(db, tenant_id).await
}

pub async fn count_clients_all(db: &Db) -> Result<i64, sqlx::Error> {
    client_repo::count_clients_all(db).await
}

pub async fn find_client_by_id(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
) -> Result<Option<Client>, sqlx::Error> {
    client_repo::find_client_by_id(db, tenant_id, client_id).await
}

pub async fn create_client(
    db: &Db,
    tenant_id: i64,
    form: ClientForm,
) -> Result<(), ClientError> {
    let (plan_key, limits) = workspace_service::plan_limits_for_tenant(db, tenant_id).await;
    if let Some(limit) = limits.clients {
        let existing = client_repo::count_clients(db, tenant_id).await.unwrap_or(0);
        if existing >= limit {
            let plan_name = workspace_service::plan_name(&plan_key);
            return Err(ClientError {
                message: format!(
                    "{plan_name} plan workspaces can have up to {limit} clients. Upgrade to add more."
                ),
                form: ClientFormView::new(
                    form.company_name,
                    form.address,
                    form.phone,
                    form.email,
                    form.latitude,
                    form.longitude,
                    form.stage,
                    form.currency,
                ),
            });
        }
    }
    let company_name = form.company_name.trim().to_string();
    if company_name.is_empty() {
        return Err(ClientError {
            message: "Company name is required.".to_string(),
            form: ClientFormView::new(
                "",
                form.address,
                form.phone,
                form.email,
                form.latitude,
                form.longitude,
                form.stage,
                form.currency,
            ),
        });
    }
    let stage = form.stage.trim().to_string();
    if stage.is_empty() {
        return Err(ClientError {
            message: "Client stage is required.".to_string(),
            form: ClientFormView::new(
                company_name,
                form.address,
                form.phone,
                form.email,
                form.latitude,
                form.longitude,
                "",
                form.currency,
            ),
        });
    }
    let currency = form.currency.trim().to_string();
    if currency.is_empty() || !currency_options().iter().any(|option| option.eq(&currency)) {
        return Err(ClientError {
            message: "Client currency is required.".to_string(),
            form: ClientFormView::new(
                company_name,
                form.address,
                form.phone,
                form.email,
                form.latitude,
                form.longitude,
                stage,
                currency,
            ),
        });
    }

    if let Err(err) = client_repo::create_client(
        db,
        tenant_id,
        &company_name,
        form.address.trim(),
        form.phone.trim(),
        form.email.trim(),
        form.latitude.trim(),
        form.longitude.trim(),
        stage.trim(),
        currency.trim(),
    )
    .await
    {
        return Err(ClientError {
            message: format!("Unable to create client: {err}"),
            form: ClientFormView::new(
                company_name,
                form.address,
                form.phone,
                form.email,
                form.latitude,
                form.longitude,
                stage,
                currency,
            ),
        });
    }

    Ok(())
}

pub async fn update_client(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
    form: ClientForm,
) -> Result<(), ClientError> {
    let company_name = form.company_name.trim().to_string();
    if company_name.is_empty() {
        return Err(ClientError {
            message: "Company name is required.".to_string(),
            form: ClientFormView::new(
                "",
                form.address,
                form.phone,
                form.email,
                form.latitude,
                form.longitude,
                form.stage,
                form.currency,
            ),
        });
    }
    let stage = form.stage.trim().to_string();
    if stage.is_empty() {
        return Err(ClientError {
            message: "Client stage is required.".to_string(),
            form: ClientFormView::new(
                company_name,
                form.address,
                form.phone,
                form.email,
                form.latitude,
                form.longitude,
                "",
                form.currency,
            ),
        });
    }
    let currency = form.currency.trim().to_string();
    if currency.is_empty() || !currency_options().iter().any(|option| option.eq(&currency)) {
        return Err(ClientError {
            message: "Client currency is required.".to_string(),
            form: ClientFormView::new(
                company_name,
                form.address,
                form.phone,
                form.email,
                form.latitude,
                form.longitude,
                stage,
                currency,
            ),
        });
    }

    if let Err(err) = client_repo::update_client(
        db,
        tenant_id,
        client_id,
        &company_name,
        form.address.trim(),
        form.phone.trim(),
        form.email.trim(),
        form.latitude.trim(),
        form.longitude.trim(),
        stage.trim(),
        currency.trim(),
    )
    .await
    {
        return Err(ClientError {
            message: format!("Unable to update client: {err}"),
            form: ClientFormView::new(
                company_name,
                form.address,
                form.phone,
                form.email,
                form.latitude,
                form.longitude,
                stage,
                currency,
            ),
        });
    }

    Ok(())
}

pub async fn delete_client(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
) -> Result<(), String> {
    client_repo::delete_contacts_by_client(db, tenant_id, client_id)
        .await
        .map_err(|err| format!("Unable to mark contacts as rogues: {err}"))?;
    client_repo::mark_client_deleted(db, tenant_id, client_id)
        .await
        .map_err(|err| format!("Unable to delete client: {err}"))?;
    Ok(())
}

pub async fn list_contacts(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
) -> Result<Vec<ClientContact>, sqlx::Error> {
    client_repo::list_contacts(db, tenant_id, client_id).await
}

pub async fn list_contacts_paged(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<ClientContact>, sqlx::Error> {
    client_repo::list_contacts_paged(db, tenant_id, client_id, limit, offset).await
}

pub async fn count_contacts(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
) -> Result<i64, sqlx::Error> {
    client_repo::count_contacts(db, tenant_id, client_id).await
}

pub async fn find_contact_by_id(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
    contact_id: i64,
) -> Result<Option<ClientContact>, sqlx::Error> {
    client_repo::find_contact_by_id(db, tenant_id, client_id, contact_id).await
}

pub async fn create_contact(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
    form: ClientContactForm,
) -> Result<(), ContactError> {
    let (plan_key, limits) = workspace_service::plan_limits_for_tenant(db, tenant_id).await;
    if let Some(limit) = limits.contacts_per_client {
        let existing = client_repo::count_contacts(db, tenant_id, client_id)
            .await
            .unwrap_or(0);
        if existing >= limit {
            let plan_name = workspace_service::plan_name(&plan_key);
            return Err(ContactError {
                message: format!(
                    "{plan_name} plan workspaces can have up to {limit} contacts per client. Upgrade to add more."
                ),
                form: ClientContactFormView::new(
                    form.name,
                    form.address,
                    form.email,
                    form.phone,
                    form.department,
                    form.position,
                ),
            });
        }
    }
    let name = form.name.trim().to_string();
    if name.is_empty() {
        return Err(ContactError {
            message: "Contact name is required.".to_string(),
            form: ClientContactFormView::new(
                "",
                form.address,
                form.email,
                form.phone,
                form.department,
                form.position,
            ),
        });
    }

    if let Err(err) = client_repo::create_contact(
        db,
        tenant_id,
        client_id,
        &name,
        form.address.trim(),
        form.email.trim(),
        form.phone.trim(),
        form.department.trim(),
        form.position.trim(),
    )
    .await
    {
        return Err(ContactError {
            message: format!("Unable to create contact: {err}"),
            form: ClientContactFormView::new(
                name,
                form.address,
                form.email,
                form.phone,
                form.department,
                form.position,
            ),
        });
    }

    Ok(())
}

pub async fn update_contact(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
    contact_id: i64,
    form: ClientContactForm,
) -> Result<(), ContactError> {
    let name = form.name.trim().to_string();
    if name.is_empty() {
        return Err(ContactError {
            message: "Contact name is required.".to_string(),
            form: ClientContactFormView::new(
                "",
                form.address,
                form.email,
                form.phone,
                form.department,
                form.position,
            ),
        });
    }

    if let Err(err) = client_repo::update_contact(
        db,
        tenant_id,
        client_id,
        contact_id,
        &name,
        form.address.trim(),
        form.email.trim(),
        form.phone.trim(),
        form.department.trim(),
        form.position.trim(),
    )
    .await
    {
        return Err(ContactError {
            message: format!("Unable to update contact: {err}"),
            form: ClientContactFormView::new(
                name,
                form.address,
                form.email,
                form.phone,
                form.department,
                form.position,
            ),
        });
    }

    Ok(())
}

pub async fn delete_contact(
    db: &Db,
    tenant_id: i64,
    client_id: i64,
    contact_id: i64,
) -> Result<(), String> {
    client_repo::delete_contact(db, tenant_id, client_id, contact_id)
        .await
        .map_err(|err| format!("Unable to delete contact: {err}"))
}
