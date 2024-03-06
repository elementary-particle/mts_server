mod api;
mod auth;
mod graphql;
mod repo;
mod schema;

use std::env;

use auth::AuthRwLock;
use axum::{extract::FromRef, http::Method};
use diesel::{r2d2::ConnectionManager, PgConnection};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use graphql::Schema;
use tower_http;
use tower_http::cors::{AllowCredentials, AllowHeaders, AllowOrigin, CorsLayer};
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

#[derive(Clone)]
struct AppState {
    repo: repo::Repo,
    auth: AuthRwLock,
    schema: Schema,
}

impl FromRef<AppState> for repo::Repo {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.repo.clone()
    }
}

impl FromRef<AppState> for AuthRwLock {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.auth.clone()
    }
}

impl FromRef<AppState> for Schema {
    fn from_ref(app_state: &AppState) -> Self {
        app_state.schema.clone()
    }
}

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations");

fn run_migrations(
    connection: &mut impl MigrationHarness<diesel::pg::Pg>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    connection.run_pending_migrations(MIGRATIONS)?;

    Ok(())
}

pub type ConnectionPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info");
    }

    env_logger::init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL");

    let admin_pass = env::var("INIT_PASS").expect("INIT_PASS");
    let host = env::var("HOST").unwrap_or(String::from("0.0.0.0"));
    let port = env::var("PORT").unwrap_or(String::from("8000"));
    let listen_addr = format!("{}:{}", host, port);

    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = r2d2::Pool::builder().build(manager).unwrap();

    run_migrations(&mut pool.get().unwrap()).unwrap();

    let app_state = AppState {
        repo: repo::Repo::new(pool),
        auth: AuthRwLock::new(),
        schema: graphql::create_schema(),
    };

    let _ = auth::service::create_user(app_state.repo.clone(), "admin", &admin_pass, true);

    let app = axum::Router::new()
        .nest(
            "/api",
            api::build_router().nest("/auth", auth::service::build_router()),
        )
        .nest("/graphql", graphql::build_router())
        .with_state(app_state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(Level::INFO))
                .on_response(DefaultOnResponse::new().level(Level::INFO)),
        )
        .layer(
            CorsLayer::new()
                .allow_methods([Method::GET, Method::POST])
                .allow_credentials(AllowCredentials::yes())
                .allow_headers(AllowHeaders::mirror_request())
                .allow_origin(AllowOrigin::mirror_request()),
        );
    let listener = tokio::net::TcpListener::bind(listen_addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}
