use actix_web::web;
use diesel::RunQueryDsl;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{api::ApiError, model, DbPool};

use crate::schema::project;

#[derive(Debug, Serialize)]
struct Project {
    pub id: Uuid,
    pub name: String,
}

#[actix_web::get("/project")]
pub async fn list(pool: web::Data<DbPool>) -> Result<web::Json<Vec<Project>>, ApiError> {
    let mut conn = pool.get().map_err(|_| ApiError::ServerError)?;

    let result = web::block(move || project::table.load::<model::Project>(&mut conn))
        .await
        .map_err(|_| ApiError::ServerError)?;

    match result {
        Ok(project_list) => Ok(web::Json(
            project_list
                .into_iter()
                .map(|t| Project {
                    id: t.id,
                    name: t.name,
                })
                .collect::<Vec<_>>(),
        )),
        Err(err) => Err(ApiError::BadRequest {
            message: err.to_string(),
        }),
    }
}

#[derive(Debug, Deserialize)]
struct NewProject {
    pub name: String,
}

#[actix_web::post("/project")]
pub async fn add(
    pool: web::Data<DbPool>,
    new_project: web::Json<NewProject>,
) -> Result<web::Json<Uuid>, ApiError> {
    let mut conn = pool.get().map_err(|_| ApiError::ServerError)?;

    let project_id = Uuid::new_v4();
    let project = model::Project {
        id: project_id,
        name: new_project.into_inner().name,
    };

    let result = web::block(move || {
        diesel::insert_into(project::table)
            .values(&project)
            .execute(&mut conn)
    })
    .await
    .map_err(|_| ApiError::ServerError)?;

    match result {
        Ok(_) => Ok(web::Json(project_id)),
        Err(err) => Err(ApiError::BadRequest {
            message: err.to_string(),
        }),
    }
}
