mod query;

use actix_web::web;
use actix_web::HttpResponse;

use juniper::http::GraphQLRequest;
use juniper::{EmptyMutation, EmptySubscription, RootNode};

use crate::auth::Claim;
use crate::repo;

pub struct Context {
    pub repo: repo::Repo,
    pub claim: Claim,
}

impl juniper::Context for Context {}

pub struct QueryRoot;

pub type Schema = RootNode<'static, QueryRoot, EmptyMutation<Context>, EmptySubscription<Context>>;

#[actix_web::get("/graphql")]
async fn panel() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(juniper::http::graphiql::graphiql_source("/graphql", None))
}

#[actix_web::post("/graphql")]
async fn index(
    schema: web::Data<Schema>,
    claim: Claim,
    repo: web::Data<repo::Repo>,
    data: web::Json<GraphQLRequest>,
) -> HttpResponse {
    let ctx = Context {
        repo: repo.get_ref().clone(),
        claim: claim,
    };
    HttpResponse::Ok()
        .content_type("application/json")
        .json(data.execute(&schema, &ctx).await)
}

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(panel).service(index);
}

pub fn create_schema() -> Schema {
    Schema::new(QueryRoot, EmptyMutation::new(), EmptySubscription::new())
}

#[derive(Debug)]
struct ServiceError;

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "An error occurred")
    }
}

impl std::error::Error for ServiceError {}
