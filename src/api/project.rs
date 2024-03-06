use axum::extract::{FromRef, Query, State};
use axum::{routing, Json, Router};
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
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Project {
    pub id: Uuid,
    pub name: String,
}

async fn get_list(State(repo): State<repo::Repo>) -> Result<Json<Vec<Project>>, ServiceError> {
    let project_list = repo.get_project()?;

    Ok(Json(
        project_list
            .into_iter()
            .map(|t| Project {
                id: t.id,
                name: t.name,
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
) -> Result<Json<Project>, ServiceError> {
    let project = repo.get_project_by_id(query.id)?;

    Ok(Json(Project {
        id: project.id,
        name: project.name,
    }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewProject {
    pub name: String,
}

async fn add(
    State(repo): State<repo::Repo>,
    _claim: Claim,
    Json(new_project): Json<NewProject>,
) -> Result<Json<Uuid>, ServiceError> {
    let project_id = Uuid::new_v4();
    let project = repo::Project {
        id: project_id,
        name: new_project.name,
    };

    repo.add_project(project)?;

    Ok(Json(project_id))
}
