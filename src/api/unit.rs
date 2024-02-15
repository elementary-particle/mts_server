use actix_web::web;
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{api::ApiError, model, ConnectionPool};

use crate::schema::{source, unit};

#[derive(Debug, Deserialize)]
struct ProjectRef {
    pub project: Uuid,
}

#[derive(Debug, Serialize)]
struct Unit {
    pub id: Uuid,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit: Option<Uuid>,
}

#[actix_web::get("/unit")]
pub async fn list(
    pool: web::Data<ConnectionPool>,
    project_ref: web::Query<ProjectRef>,
) -> Result<web::Json<Vec<Unit>>, ApiError> {
    let mut conn = pool.get().map_err(|_| ApiError::ServerError)?;

    let result = web::block(move || {
        unit::table
            .filter(unit::dsl::project_id.eq(project_ref.into_inner().project))
            .order_by(unit::dsl::title)
            .load::<model::Unit>(&mut conn)
    })
    .await
    .map_err(|_| ApiError::ServerError)?;

    match result {
        Ok(ref unit_list) => println!("{}", unit_list.len()),
        Err(_) => (),
    };
    match result {
        Ok(unit_list) => Ok(web::Json(
            unit_list
                .into_iter()
                .map(|t| Unit {
                    id: t.id,
                    title: t.title,
                    commit: t.commit_id,
                })
                .collect::<Vec<_>>(),
        )),
        Err(err) => Err(ApiError::BadRequest {
            message: err.to_string(),
        }),
    }
}

#[derive(Debug, Deserialize)]
struct UnitRef {
    pub id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
struct Source {
    pub sq: i32,
    pub content: String,
    pub meta: String,
}

#[actix_web::get("/unit/source")]
pub async fn sources(
    pool: web::Data<ConnectionPool>,
    unit_ref: web::Query<UnitRef>,
) -> Result<web::Json<Vec<Source>>, ApiError> {
    let mut conn = pool.get().map_err(|_| ApiError::ServerError)?;

    let unit_id = unit_ref.into_inner().id;

    let result = web::block(move || {
        source::table
            .filter(source::dsl::unit_id.eq(unit_id))
            .load::<model::Source>(&mut conn)
    })
    .await
    .map_err(|_| ApiError::ServerError)?;

    match result {
        Ok(record_list) => Ok(web::Json(
            record_list
                .into_iter()
                .map(|t: model::Source| Source {
                    sq: t.sq,
                    content: t.content,
                    meta: t.meta,
                })
                .collect::<Vec<_>>(),
        )),
        Err(err) => Err(ApiError::BadRequest {
            message: err.to_string(),
        }),
    }
}

#[derive(Debug, Deserialize)]
struct NewUnit {
    pub project: Uuid,
    pub title: String,
    #[serde(rename = "sourceList")]
    pub source_list: Vec<Source>,
}

#[actix_web::post("/unit")]
pub async fn add(
    pool: web::Data<ConnectionPool>,
    new_unit: web::Json<NewUnit>,
) -> Result<web::Json<Uuid>, ApiError> {
    let mut conn = pool.get().map_err(|_| ApiError::ServerError)?;

    let new_unit = new_unit.into_inner();
    let unit_id = Uuid::new_v4();

    let result = web::block(move || {
        diesel::insert_into(unit::table)
            .values(model::Unit {
                project_id: new_unit.project,
                id: unit_id,
                title: new_unit.title,
                commit_id: None,
            })
            .execute(&mut conn)?;
        diesel::insert_into(source::dsl::source)
            .values(
                new_unit
                    .source_list
                    .into_iter()
                    .map(|t| model::Source {
                        unit_id: unit_id,
                        sq: t.sq,
                        content: t.content,
                        meta: t.meta,
                    })
                    .collect::<Vec<_>>(),
            )
            .execute(&mut conn)
    })
    .await
    .map_err(|_| ApiError::ServerError)?;

    match result {
        Ok(_) => Ok(web::Json(unit_id)),
        Err(err) => Err(ApiError::BadRequest {
            message: err.to_string(),
        }),
    }
}
