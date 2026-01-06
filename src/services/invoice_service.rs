use rocket_db_pools::sqlx;

use crate::models::{DeploymentSelect, Invoice, InvoiceCandidate, InvoiceForm, InvoiceFormView, InvoiceSummary};
use crate::repositories::{deployment_repo, invoice_repo};
use crate::Db;

pub struct InvoiceError {
    pub message: String,
    pub form: InvoiceFormView,
}

const STATUS_DRAFT: &str = "Draft";
const STATUS_SENT: &str = "Sent";
const STATUS_PAID: &str = "Paid";

pub fn status_options() -> [&'static str; 3] {
    [STATUS_DRAFT, STATUS_SENT, STATUS_PAID]
}

pub async fn list_invoices_with_details(
    db: &Db,
    tenant_id: i64,
) -> Result<Vec<InvoiceSummary>, sqlx::Error> {
    invoice_repo::list_invoices_with_details(db, tenant_id).await
}

pub async fn find_invoice_with_details(
    db: &Db,
    tenant_id: i64,
    invoice_id: i64,
) -> Result<Option<InvoiceSummary>, sqlx::Error> {
    invoice_repo::find_invoice_with_details(db, tenant_id, invoice_id).await
}

pub async fn find_invoice_by_id(
    db: &Db,
    tenant_id: i64,
    invoice_id: i64,
) -> Result<Option<Invoice>, sqlx::Error> {
    invoice_repo::find_invoice_by_id(db, tenant_id, invoice_id).await
}

pub async fn list_invoice_candidates(
    db: &Db,
    tenant_id: i64,
) -> Result<Vec<InvoiceCandidate>, sqlx::Error> {
    invoice_repo::list_completed_deployments_without_invoice(db, tenant_id).await
}

pub async fn list_invoice_candidates_for_select(
    db: &Db,
    tenant_id: i64,
) -> Result<Vec<DeploymentSelect>, sqlx::Error> {
    let candidates = list_invoice_candidates(db, tenant_id).await?;
    Ok(candidates
        .into_iter()
        .map(|candidate| DeploymentSelect {
            id: candidate.deployment_id,
            label: format!(
                "{} - {} ({} to {})",
                candidate.client_name, candidate.crew_name, candidate.start_at, candidate.end_at
            ),
        })
        .collect())
}

pub async fn create_invoice(
    db: &Db,
    tenant_id: i64,
    form: InvoiceForm,
) -> Result<i64, InvoiceError> {
    let deployment_id = form.deployment_id;
    let status = normalize_status(form.status);
    let notes = form.notes.trim().to_string();

    if deployment_id <= 0 {
        return Err(InvoiceError {
            message: "Deployment is required.".to_string(),
            form: InvoiceFormView::new(0, status, notes),
        });
    }

    let deployment = deployment_repo::find_deployment_by_id(db, tenant_id, deployment_id)
        .await
        .map_err(|err| InvoiceError {
            message: format!("Unable to load deployment: {err}"),
            form: InvoiceFormView::new(deployment_id, status.clone(), notes.clone()),
        })?;
    let deployment = match deployment {
        Some(deployment) => deployment,
        None => {
            return Err(InvoiceError {
                message: "Deployment not found.".to_string(),
                form: InvoiceFormView::new(deployment_id, status, notes),
            })
        }
    };

    if !deployment.status.eq_ignore_ascii_case("Completed") {
        return Err(InvoiceError {
            message: "Only completed deployments can be invoiced.".to_string(),
            form: InvoiceFormView::new(deployment_id, status, notes),
        });
    }

    if let Ok(Some(_)) = invoice_repo::find_invoice_by_deployment(db, tenant_id, deployment_id).await {
        return Err(InvoiceError {
            message: "An invoice already exists for this deployment.".to_string(),
            form: InvoiceFormView::new(deployment_id, status, notes),
        });
    }

    invoice_repo::create_invoice(db, tenant_id, deployment_id, &status, &notes)
        .await
        .map_err(|err| InvoiceError {
            message: format!("Unable to create invoice: {err}"),
            form: InvoiceFormView::new(deployment_id, status, notes),
        })
}

pub async fn update_invoice(
    db: &Db,
    tenant_id: i64,
    invoice_id: i64,
    form: InvoiceForm,
) -> Result<(), InvoiceError> {
    let deployment_id = form.deployment_id;
    let status = normalize_status(form.status);
    let notes = form.notes.trim().to_string();

    let invoice = invoice_repo::find_invoice_by_id(db, tenant_id, invoice_id)
        .await
        .map_err(|err| InvoiceError {
            message: format!("Unable to load invoice: {err}"),
            form: InvoiceFormView::new(deployment_id, status.clone(), notes.clone()),
        })?;
    let invoice = match invoice {
        Some(invoice) => invoice,
        None => {
            return Err(InvoiceError {
                message: "Invoice not found.".to_string(),
                form: InvoiceFormView::new(deployment_id, status, notes),
            })
        }
    };

    if deployment_id != invoice.deployment_id {
        return Err(InvoiceError {
            message: "Deployment cannot be changed for an invoice.".to_string(),
            form: InvoiceFormView::new(invoice.deployment_id, status, notes),
        });
    }

    invoice_repo::update_invoice(db, tenant_id, invoice_id, &status, &notes)
        .await
        .map_err(|err| InvoiceError {
            message: format!("Unable to update invoice: {err}"),
            form: InvoiceFormView::new(invoice.deployment_id, status, notes),
        })?;

    Ok(())
}

pub async fn delete_invoice(db: &Db, tenant_id: i64, invoice_id: i64) -> Result<(), String> {
    invoice_repo::delete_invoice(db, tenant_id, invoice_id)
        .await
        .map_err(|err| format!("Unable to delete invoice: {err}"))
}

pub async fn count_invoices(db: &Db, tenant_id: i64) -> Result<i64, sqlx::Error> {
    invoice_repo::count_invoices(db, tenant_id).await
}

pub async fn count_invoices_all(db: &Db) -> Result<i64, sqlx::Error> {
    invoice_repo::count_invoices_total(db).await
}

pub async fn count_invoices_by_status(
    db: &Db,
    tenant_id: i64,
) -> Result<Vec<(String, i64)>, sqlx::Error> {
    invoice_repo::count_invoices_by_status(db, tenant_id).await
}

pub async fn count_invoices_by_status_all(
    db: &Db,
) -> Result<Vec<(String, i64)>, sqlx::Error> {
    invoice_repo::count_invoices_by_status_all(db).await
}

fn normalize_status(input: String) -> String {
    let trimmed = input.trim();
    for option in status_options() {
        if option.eq_ignore_ascii_case(trimmed) {
            return option.to_string();
        }
    }
    STATUS_DRAFT.to_string()
}
