use actix_web::web;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::ApiError;
use crate::auth::Claim;
use crate::repo;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Project {
    pub id: Uuid,
    pub name: String,
}

#[actix_web::get("/project")]
pub async fn list(repo: web::Data<repo::Repo>) -> Result<web::Json<Vec<Project>>, ApiError> {
    let project_list = web::block(move || repo.get_project())
        .await
        .map_err(|_| ApiError::ServerError)?
        .ok_or(ApiError::BadRequest)?;

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
#[serde(rename_all = "camelCase")]
struct NewProject {
    pub name: String,
}

#[actix_web::post("/project")]
pub async fn add(
    claim: Claim,
    repo: web::Data<repo::Repo>,
    new_project: web::Json<NewProject>,
) -> Result<web::Json<Uuid>, ApiError> {
    match claim {
        Claim::Guest => Err(ApiError::Unauthorized),
        Claim::User { .. } => Ok(()),
    }?;

    let project_id = Uuid::new_v4();
    let project = repo::Project {
        id: project_id,
        name: new_project.into_inner().name,
    };

    web::block(move || repo.add_project(project))
        .await
        .map_err(|_| ApiError::ServerError)?
        .ok_or(ApiError::BadRequest)?;

    Ok(web::Json(project_id))
}
