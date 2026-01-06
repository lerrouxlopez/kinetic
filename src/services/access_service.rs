use std::collections::HashMap;

use rocket_db_pools::sqlx;

use crate::models::{User, UserPermission};
use crate::repositories::{user_permission_repo, user_repo};
use crate::Db;

pub const RESOURCES: [(&str, &str); 7] = [
    ("dashboard", "Dashboard"),
    ("clients", "Clients"),
    ("crew", "Crew"),
    ("deployments", "Deployments"),
    ("tracking", "Tracking"),
    ("invoices", "Invoices"),
    ("settings", "Settings"),
];

const ROLE_OWNER: &str = "Owner";
const ROLE_ADMIN: &str = "Admin";
const ROLE_SALES: &str = "Sales";
const ROLE_OPERATIONS: &str = "Operations";
const ROLE_ACCOUNTING: &str = "Accounting";
const ROLE_EMPLOYEE: &str = "Employee";

pub fn role_options() -> [&'static str; 6] {
    [
        ROLE_OWNER,
        ROLE_ADMIN,
        ROLE_SALES,
        ROLE_OPERATIONS,
        ROLE_ACCOUNTING,
        ROLE_EMPLOYEE,
    ]
}

pub fn is_owner(role: &str) -> bool {
    normalize_role(role).eq_ignore_ascii_case(ROLE_OWNER)
}

pub fn is_admin(role: &str) -> bool {
    normalize_role(role).eq_ignore_ascii_case(ROLE_ADMIN)
}

pub fn is_employee(role: &str) -> bool {
    normalize_role(role).eq_ignore_ascii_case(ROLE_EMPLOYEE)
}

pub async fn list_permissions_for_user(
    db: &Db,
    tenant_id: i64,
    user_id: i64,
    role: &str,
) -> Result<Vec<UserPermission>, sqlx::Error> {
    let existing = user_permission_repo::list_permissions_for_user(db, tenant_id, user_id).await?;
    if !existing.is_empty() {
        return Ok(existing);
    }

    let defaults = default_permissions_for_role(role);
    user_permission_repo::replace_permissions_for_user(db, tenant_id, user_id, &defaults).await?;
    Ok(defaults)
}

pub async fn replace_permissions_for_user(
    db: &Db,
    tenant_id: i64,
    user_id: i64,
    permissions: &[UserPermission],
) -> Result<(), sqlx::Error> {
    user_permission_repo::replace_permissions_for_user(db, tenant_id, user_id, permissions).await
}

pub async fn can_view(db: &Db, user: &User, resource: &str) -> bool {
    can_action(db, user, resource, "view").await
}

pub async fn can_edit(db: &Db, user: &User, resource: &str) -> bool {
    can_action(db, user, resource, "edit").await
}

pub async fn can_delete(db: &Db, user: &User, resource: &str) -> bool {
    can_action(db, user, resource, "delete").await
}

async fn can_action(db: &Db, user: &User, resource: &str, action: &str) -> bool {
    let role = normalize_role(&user.role);
    if role.eq_ignore_ascii_case(ROLE_OWNER) {
        return true;
    }
    if role.eq_ignore_ascii_case(ROLE_ADMIN) {
        return action != "delete";
    }

    let permissions = user_permission_repo::list_permissions_for_user(db, user.tenant_id, user.id)
        .await
        .unwrap_or_default();
    let mut by_resource: HashMap<String, UserPermission> = permissions
        .into_iter()
        .map(|perm| (perm.resource.clone(), perm))
        .collect();

    if by_resource.is_empty() {
        by_resource = default_permissions_for_role(role)
            .into_iter()
            .map(|perm| (perm.resource.clone(), perm))
            .collect();
    }

    match by_resource.get(resource) {
        Some(permission) => match action {
            "view" => permission.can_view,
            "edit" => permission.can_edit,
            "delete" => permission.can_delete,
            _ => false,
        },
        None => false,
    }
}

pub async fn refresh_user_role(
    db: &Db,
    tenant_id: i64,
    user_id: i64,
    role: &str,
) -> Result<(), sqlx::Error> {
    user_repo::update_user_role(db, tenant_id, user_id, role).await?;
    Ok(())
}

fn default_permissions_for_role(role: &str) -> Vec<UserPermission> {
    let mut perms = Vec::new();
    let normalized = normalize_role(role);

    for (resource, _) in RESOURCES.iter() {
        let (can_view, can_edit, can_delete) = match normalized {
            ROLE_OWNER => (true, true, true),
            ROLE_ADMIN => (true, true, false),
            ROLE_SALES => match *resource {
                "dashboard" => (true, false, false),
                "clients" => (true, true, false),
                "deployments" => (true, true, false),
                "tracking" => (true, true, false),
                _ => (false, false, false),
            },
            ROLE_OPERATIONS => match *resource {
                "dashboard" => (true, false, false),
                "crew" => (true, true, false),
                "deployments" => (true, true, false),
                "tracking" => (true, true, false),
                _ => (false, false, false),
            },
            ROLE_ACCOUNTING => match *resource {
                "dashboard" => (true, false, false),
                "invoices" => (true, false, false),
                _ => (false, false, false),
            },
            ROLE_EMPLOYEE => match *resource {
                "dashboard" => (true, false, false),
                "deployments" => (true, false, false),
                "tracking" => (true, true, false),
                _ => (false, false, false),
            },
            _ => (false, false, false),
        };

        perms.push(UserPermission {
            resource: (*resource).to_string(),
            can_view,
            can_edit,
            can_delete,
        });
    }

    perms
}

fn normalize_role(role: &str) -> &str {
    let trimmed = role.trim();
    if trimmed.is_empty() {
        return ROLE_OWNER;
    }
    for option in role_options() {
        if option.eq_ignore_ascii_case(trimmed) {
            return option;
        }
    }
    trimmed
}
