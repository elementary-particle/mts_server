mod api;
mod auth;
mod graphql;
mod repo;
mod schema;

use std::sync::{Arc, Mutex};
use std::{env, io};

use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::web;
use actix_web::{App, HttpServer};
use diesel::{r2d2::ConnectionManager, PgConnection};

pub type ConnectionPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[actix_rt::main]
async fn main() -> io::Result<()> {
    dotenvy::dotenv().ok();

    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "actix_web=debug,actix_server=info");
    }

    env_logger::init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL");
    let admin_pass = env::var("INIT_PASS").expect("INIT_PASS");
    let host = env::var("HOST").unwrap_or(String::from("0.0.0.0"));
    let port = env::var("PORT").unwrap_or(String::from("8000"));
    let listen_addr = format!("{}:{}", host, port);

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder().build(manager).unwrap();
    let repo = repo::Repo::new(pool);
    let secret = Arc::new(Mutex::new(auth::Secret::new()));
    let schema = Arc::new(graphql::create_schema());

    let _ = auth::service::create_user(repo.clone(), "admin", &admin_pass, true);

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::new("%a \"%r\" %s %b %T"))
            .wrap(Cors::permissive())
            .app_data(web::Data::from(secret.clone()))
            .app_data(web::Data::new(repo.clone()))
            .service(
                web::scope("/api")
                    .service(web::scope("/auth").configure(auth::service::configure))
                    .service(api::project::list)
                    .service(api::project::get_by_id)
                    .service(api::project::add)
                    .service(api::unit::get_list)
                    .service(api::unit::get_by_id)
                    .service(api::unit::get_source_list)
                    .service(api::unit::add)
                    .service(api::commit::list)
                    .service(api::commit::get_by_id)
                    .service(api::commit::get_record_list)
                    .service(api::commit::add),
            )
            .service(
                web::scope("")
                    .app_data(web::Data::from(schema.clone()))
                    .configure(graphql::configure),
            )
    })
    .bind(listen_addr)?
    .run()
    .await
}
