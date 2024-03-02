use actix_web::web;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::api::ServiceError;
use crate::auth::Claim;
use crate::repo;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
struct ProjectQuery {
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

#[actix_web::get("/unit")]
pub async fn get_list(
    repo: web::Data<repo::Repo>,
    project_query: web::Query<ProjectQuery>,
) -> Result<web::Json<Vec<Unit>>, ServiceError> {
    let unit_list =
        web::block(move || repo.get_unit_by_project_id(project_query.into_inner().project_id))
            .await??;

    Ok(web::Json(
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
struct UnitQuery {
    pub id: Uuid,
}

#[actix_web::get("/unit/by-id")]
pub async fn get_by_id(
    repo: web::Data<repo::Repo>,
    unit_query: web::Query<UnitQuery>,
) -> Result<web::Json<Unit>, ServiceError> {
    let unit = web::block(move || repo.get_unit_by_id(unit_query.into_inner().id)).await??;

    Ok(web::Json(Unit {
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

#[actix_web::get("/unit/source")]
pub async fn get_source_list(
    repo: web::Data<repo::Repo>,
    unit_query: web::Query<UnitQuery>,
) -> Result<web::Json<Vec<Source>>, ServiceError> {
    let source_list =
        web::block(move || repo.get_source_by_unit_id(unit_query.into_inner().id)).await??;

    Ok(web::Json(
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

#[actix_web::post("/unit")]
pub async fn add(
    _claim: Claim,
    repo: web::Data<repo::Repo>,
    new_unit: web::Json<NewUnit>,
) -> Result<web::Json<Uuid>, ServiceError> {
    let new_unit = new_unit.into_inner();
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

    web::block(move || repo.add_unit(unit, source_list)).await??;

    Ok(web::Json(unit_id))
}
