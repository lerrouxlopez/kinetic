#[macro_use]
extern crate rocket;

mod controllers;
mod models;
mod repositories;
mod services;

use rocket::fairing::AdHoc;
use rocket::fs::{relative, FileServer};
use rocket_db_pools::{sqlx, Database};
use rocket_dyn_templates::Template;

use controllers::admin_controller::{
    admin_login_form,
    admin_login_submit,
    admin_logout,
    admin_plans,
    admin_plans_update,
    admin_plans_update_enterprise,
    admin_plans_update_pro,
    admin_user_create,
    admin_user_delete,
    admin_user_edit_form,
    admin_user_new_form,
    admin_user_update,
    admin_users,
    admin_workspaces,
    admin_workspace_create,
    admin_workspace_delete,
    admin_workspace_expire,
    admin_workspace_edit_form,
    admin_workspace_new_form,
    admin_workspace_update,
};
use controllers::crew_controller::{
    crew_create,
    crew_delete,
    crew_edit_form,
    crew_index,
    crew_new_form,
    crew_show,
    crew_member_new_form,
    crew_member_create,
    crew_member_edit_form,
    crew_member_update,
    crew_member_delete,
    crew_update,
};
use controllers::invoice_controller::{
    invoice_create,
    invoice_delete,
    invoice_edit_form,
    invoice_email_form,
    invoice_email_send,
    invoice_new_form,
    invoice_show,
    invoice_update,
    invoices_index,
};
use controllers::client_controller::{
    client_create,
    client_delete,
    client_edit_form,
    client_new_form,
    client_show,
    client_update,
    clients_index,
    client_email_blast_form,
    client_email_blast_send,
    appointment_create,
    appointment_delete,
    appointment_edit_form,
    appointment_new_form,
    appointment_update,
    contact_create,
    contact_delete,
    contact_email_form,
    contact_email_send,
    contact_edit_form,
    contact_new_form,
    contact_update,
};
use controllers::public_controller::{
    dashboard,
    deployment_delete,
    deployment_edit_form,
    deployment_update,
    deployment_create,
    deployment_new_form,
    deployments,
    index,
    login_form,
    login_submit,
    logout,
    plans,
    register_form,
    register_submit,
    settings_users_update,
    settings,
    settings_email_update,
    settings_theme_update,
    settings_seed_demo,
    tracking,
    tracking_timer_start,
    tracking_timer_stop,
    tracking_update_create,
    tracking_update_delete,
    tracking_update_edit_form,
    tracking_update_update,
    workspace_register_form,
    workspace_register_submit,
};
use services::schema_service;

#[derive(Database)]
#[database("kinetic_db")]
pub struct Db(sqlx::SqlitePool);

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(Db::init())
        .attach(Template::fairing())
        .attach(AdHoc::try_on_ignite("Init DB", |rocket| async {
            let db = Db::fetch(&rocket).expect("database pool");
            if let Err(err) = schema_service::ensure_schema(db).await {
                eprintln!("Failed to init schema: {err}");
                return Err(rocket);
            }
            Ok(rocket)
        }))
        .mount(
            "/",
            routes![
                index,
                register_form,
                register_submit,
                login_form,
                login_submit,
                logout,
                admin_login_form,
                admin_login_submit,
                admin_logout,
                admin_plans,
                admin_plans_update,
                admin_plans_update_enterprise,
                admin_plans_update_pro,
                admin_users,
                admin_user_new_form,
                admin_user_create,
                admin_user_edit_form,
                admin_user_update,
                admin_user_delete,
                admin_workspaces,
                admin_workspace_new_form,
                admin_workspace_create,
                admin_workspace_edit_form,
                admin_workspace_update,
                admin_workspace_delete,
                admin_workspace_expire,
                workspace_register_form,
                workspace_register_submit,
                crew_index,
                crew_show,
                crew_new_form,
                crew_create,
                crew_edit_form,
                crew_update,
                crew_delete,
                crew_member_new_form,
                crew_member_create,
                crew_member_edit_form,
                crew_member_update,
                crew_member_delete,
                clients_index,
                client_new_form,
                client_create,
                client_show,
                client_email_blast_form,
                client_email_blast_send,
                client_edit_form,
                client_update,
                client_delete,
                appointment_new_form,
                appointment_create,
                appointment_edit_form,
                appointment_update,
                appointment_delete,
                contact_new_form,
                contact_create,
                contact_email_form,
                contact_email_send,
                contact_edit_form,
                contact_update,
                contact_delete,
                tracking,
                tracking_timer_start,
                tracking_timer_stop,
                tracking_update_create,
                tracking_update_edit_form,
                tracking_update_update,
                tracking_update_delete,
                invoices_index,
                invoice_new_form,
                invoice_create,
                invoice_show,
                invoice_edit_form,
                invoice_update,
                invoice_delete,
                invoice_email_form,
                invoice_email_send,
                settings,
                settings_users_update,
                settings_email_update,
                settings_theme_update,
                settings_seed_demo,
                deployment_new_form,
                deployment_create,
                deployment_edit_form,
                deployment_update,
                deployment_delete,
                deployments,
                plans,
                dashboard
            ],
        )
        .mount("/static", FileServer::from(relative!("static")))
}
