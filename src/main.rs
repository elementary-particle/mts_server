mod api;
mod model;
mod schema;

use std::{env, io};

use actix_cors::Cors;
use actix_web::web::Data;
use actix_web::{http, middleware};
use actix_web::{App, HttpServer};
use diesel::{r2d2::ConnectionManager, PgConnection};

pub type DbPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[actix_rt::main]
async fn main() -> io::Result<()> {
    dotenv::dotenv().ok();

    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "actix_web=debug,actix_server=info");
    }

    env_logger::init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder().build(manager).unwrap();

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(pool.clone()))
            .wrap(
                Cors::default()
                    .allowed_origin("http://localhost:3000")
                    .allowed_methods(vec!["GET", "POST"])
                    .allowed_header(http::header::AUTHORIZATION)
                    .allowed_header(http::header::ACCEPT)
                    .allowed_header(http::header::CONTENT_TYPE),
            )
            .wrap(middleware::Logger::default())
            .service(api::project::list)
            .service(api::project::add)
            .service(api::unit::list)
            .service(api::unit::sources)
            .service(api::unit::add)
            .service(api::commit::list)
            .service(api::commit::records)
            .service(api::commit::add)
    })
    .bind("0.0.0.0:8000")?
    .run()
    .await
}
