use actix_web::web;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::ServiceError;
use crate::auth::Claim;
use crate::repo;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Project {
    pub id: Uuid,
    pub name: String,
}

#[actix_web::get("/project")]
pub async fn list(repo: web::Data<repo::Repo>) -> Result<web::Json<Vec<Project>>, ServiceError> {
    let project_list = web::block(move || repo.get_project()).await??;

    Ok(web::Json(
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
struct ProjectQuery {
    pub id: Uuid,
}

#[actix_web::get("/project/by-id")]
pub async fn get_by_id(
    repo: web::Data<repo::Repo>,
    project_query: web::Query<ProjectQuery>,
) -> Result<web::Json<Project>, ServiceError> {
    let project =
        web::block(move || repo.get_project_by_id(project_query.into_inner().id)).await??;

    Ok(web::Json(Project {
        id: project.id,
        name: project.name,
    }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NewProject {
    pub name: String,
}

#[actix_web::post("/project")]
pub async fn add(
    _claim: Claim,
    repo: web::Data<repo::Repo>,
    new_project: web::Json<NewProject>,
) -> Result<web::Json<Uuid>, ServiceError> {
    let project_id = Uuid::new_v4();
    let project = repo::Project {
        id: project_id,
        name: new_project.into_inner().name,
    };

    web::block(move || repo.add_project(project)).await??;

    Ok(web::Json(project_id))
}
