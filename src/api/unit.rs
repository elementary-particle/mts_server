use actix_web::web;
use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::{ExpressionMethods, NullableExpressionMethods, QueryDsl, RunQueryDsl};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{api::ApiError, model, DbPool};

use crate::schema::{commit, source, unit};

#[derive(Debug, Deserialize)]
struct ProjectRef {
    pub project: Uuid,
}

#[derive(Debug, Serialize)]
struct Unit {
    pub id: Uuid,
    pub title: String,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
}

#[actix_web::get("/unit")]
pub async fn list(
    pool: web::Data<DbPool>,
    project_ref: web::Query<ProjectRef>,
) -> Result<web::Json<Vec<Unit>>, ApiError> {
    let mut conn = pool.get().map_err(|_| ApiError::ServerError)?;

    let result = web::block(move || {
        unit::table
            .filter(unit::dsl::project_id.eq(project_ref.into_inner().project))
            .left_join(commit::table)
            .group_by(unit::dsl::id)
            .select((
                unit::dsl::id,
                unit::dsl::title,
                diesel::dsl::max(commit::dsl::created_at.nullable()),
            ))
            .order_by(unit::dsl::title)
            .load::<(Uuid, String, Option<NaiveDateTime>)>(&mut conn)
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
                    id: t.0,
                    title: t.1,
                    updated_at: match t.2 {
                        Some(time) => Some(time.and_utc()),
                        None => None,
                    },
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

#[actix_web::get("/unit")]
pub async fn sources(
    pool: web::Data<DbPool>,
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
    pool: web::Data<DbPool>,
    new_unit: web::Json<NewUnit>,
) -> Result<web::Json<usize>, ApiError> {
    let mut conn = pool.get().map_err(|_| ApiError::ServerError)?;

    let new_unit = new_unit.into_inner();
    let unit_id = Uuid::new_v4();

    let result = web::block(move || {
        diesel::insert_into(unit::table)
            .values(model::Unit {
                project_id: new_unit.project,
                id: unit_id,
                title: new_unit.title,
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
        Ok(rows_affected) => Ok(web::Json(rows_affected)),
        Err(err) => Err(ApiError::BadRequest {
            message: err.to_string(),
        }),
    }
}
