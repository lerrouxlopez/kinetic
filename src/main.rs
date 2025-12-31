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
    admin_workspaces,
    admin_workspace_create,
    admin_workspace_delete,
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
    crew_update,
};
use controllers::public_controller::{
    dashboard,
    index,
    login_form,
    login_submit,
    logout,
    register_form,
    register_submit,
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
                admin_workspaces,
                admin_workspace_new_form,
                admin_workspace_create,
                admin_workspace_edit_form,
                admin_workspace_update,
                admin_workspace_delete,
                crew_index,
                crew_new_form,
                crew_create,
                crew_edit_form,
                crew_update,
                crew_delete,
                dashboard
            ],
        )
        .mount("/static", FileServer::from(relative!("static")))
}
