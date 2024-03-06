mod query;

use std::sync::Arc;

use axum::extract::{FromRef, Json, State};
use axum::http::header::CONTENT_TYPE;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::{routing, Router};
use juniper::http::{GraphQLRequest, GraphQLResponse};
use juniper::{EmptyMutation, EmptySubscription, RootNode};

use crate::auth::{AuthRwLock, Claim, OptionalClaim};
use crate::repo;

pub struct Context {
    pub repo: repo::Repo,
    pub option_claim: Option<Claim>,
}

pub fn build_router<S>() -> Router<S>
where
    S: Send + Sync + Clone + 'static,
    AuthRwLock: FromRef<S>,
    repo::Repo: FromRef<S>,
    Schema: FromRef<S>,
{
    Router::new().route("/", routing::get(graphiql).post(index))
}

impl juniper::Context for Context {}

pub struct QueryRoot;

pub type Schema =
    Arc<RootNode<'static, QueryRoot, EmptyMutation<Context>, EmptySubscription<Context>>>;

async fn graphiql() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(CONTENT_TYPE, "text/html; charset=utf-8")],
        juniper::http::graphiql::graphiql_source("/graphql", None),
    )
}

async fn index(
    State(schema): State<Schema>,
    State(repo): State<repo::Repo>,
    OptionalClaim(option): OptionalClaim,
    data: Json<GraphQLRequest>,
) -> Json<GraphQLResponse> {
    let ctx = Context {
        repo: repo.clone(),
        option_claim: option,
    };
    Json(data.execute(&schema, &ctx).await)
}

pub fn create_schema() -> Schema {
    Arc::new(RootNode::new(
        QueryRoot,
        EmptyMutation::new(),
        EmptySubscription::new(),
    ))
}

#[derive(Debug)]
struct ServiceError(repo::Error);

impl From<repo::Error> for ServiceError {
    fn from(error: repo::Error) -> Self {
        ServiceError(error)
    }
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for ServiceError {}
