use axum::extract::{FromRef, Json, Query, State};
use axum::{routing, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::ServiceError;
use crate::auth::{AuthRwLock, Claim};
use crate::repo;

pub fn build_router<S>() -> Router<S>
where
    S: Send + Sync + Clone + 'static,
    AuthRwLock: FromRef<S>,
    repo::Repo: FromRef<S>,
{
    Router::new()
        .route("/", routing::get(get_list).post(add))
        .route("/by-id", routing::get(get_by_id))
        .route("/source", routing::get(get_source_list))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ProjectIdQuery {
    pub project_id: Uuid,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Unit {
    pub id: Uuid,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_id: Option<Uuid>,
}

async fn get_list(
    State(repo): State<repo::Repo>,
    Query(query): Query<ProjectIdQuery>,
) -> Result<Json<Vec<Unit>>, ServiceError> {
    let unit_list = repo.get_unit_by_project_id(query.project_id)?;

    Ok(Json(
        unit_list
            .into_iter()
            .map(|t| Unit {
                id: t.id,
                title: t.title,
                commit_id: t.commit_id,
            })
            .collect::<Vec<_>>(),
    ))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct IdQuery {
    pub id: Uuid,
}

async fn get_by_id(
    State(repo): State<repo::Repo>,
    Query(query): Query<IdQuery>,
) -> Result<Json<Unit>, ServiceError> {
    let unit = repo.get_unit_by_id(query.id)?;

    Ok(Json(Unit {
        id: unit.id,
        title: unit.title,
        commit_id: unit.commit_id,
    }))
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Source {
    pub sq: i32,
    pub content: String,
    pub meta: String,
}

async fn get_source_list(
    State(repo): State<repo::Repo>,
    Query(query): Query<IdQuery>,
) -> Result<Json<Vec<Source>>, ServiceError> {
    let source_list = repo.get_source_by_unit_id(query.id)?;

    Ok(Json(
        source_list
            .into_iter()
            .map(|t| Source {
                sq: t.sq,
                content: t.content,
                meta: t.meta,
            })
            .collect::<Vec<_>>(),
    ))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewUnit {
    pub project_id: Uuid,
    pub title: String,
    pub source_list: Vec<Source>,
}

async fn add(
    State(repo): State<repo::Repo>,
    _claim: Claim,
    Json(new_unit): Json<NewUnit>,
) -> Result<Json<Uuid>, ServiceError> {
    let unit_id = Uuid::new_v4();
    let unit = repo::Unit {
        id: unit_id,
        project_id: new_unit.project_id,
        title: new_unit.title,
        commit_id: None,
    };
    let source_list = new_unit
        .source_list
        .into_iter()
        .map(|t| repo::Source {
            unit_id: unit_id,
            sq: t.sq,
            content: t.content,
            meta: t.meta,
        })
        .collect::<Vec<_>>();

    repo.add_unit(unit, source_list)?;

    Ok(Json(unit_id))
}
