use actix_web::web;
use chrono::{DateTime, Utc};
use diesel::query_dsl::methods::FilterDsl;
use diesel::{ExpressionMethods, RunQueryDsl};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{api::ApiError, model, DbPool};

use crate::schema::{commit, record};

#[derive(Debug, Deserialize)]
struct UnitRef {
    pub unit: Uuid,
}

#[derive(Debug, Serialize)]
struct Commit {
    pub id: Uuid,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
}

#[actix_web::get("/commit")]
pub async fn list(
    pool: web::Data<DbPool>,
    unit_ref: web::Query<UnitRef>,
) -> Result<web::Json<Vec<Commit>>, ApiError> {
    let mut conn = pool.get().map_err(|_| ApiError::ServerError)?;

    let unit_id = unit_ref.into_inner().unit;

    let result = web::block(move || {
        commit::table
            .filter(commit::dsl::unit_id.eq(unit_id))
            .load::<model::Commit>(&mut conn)
    })
    .await
    .map_err(|_| ApiError::ServerError)?;

    match result {
        Ok(commit_list) => Ok(web::Json(
            commit_list
                .into_iter()
                .map(|t| Commit {
                    id: t.id,
                    created_at: t.created_at.and_utc(),
                })
                .collect::<Vec<_>>(),
        )),
        Err(err) => Err(ApiError::BadRequest {
            message: err.to_string(),
        }),
    }
}

#[derive(Debug, Deserialize)]
struct CommitRef {
    pub id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
struct Record {
    pub sq: i32,
    pub content: String,
}

#[actix_web::get("/commit")]
pub async fn records(
    pool: web::Data<DbPool>,
    commit_ref: web::Query<CommitRef>,
) -> Result<web::Json<Vec<Record>>, ApiError> {
    let mut conn = pool.get().map_err(|_| ApiError::ServerError)?;

    let commit_id = commit_ref.into_inner().id;

    let result = web::block(move || {
        record::table
            .filter(record::dsl::commit_id.eq(commit_id))
            .load::<model::Record>(&mut conn)
    })
    .await
    .map_err(|_| ApiError::ServerError)?;

    match result {
        Ok(record_list) => Ok(web::Json(
            record_list
                .into_iter()
                .map(|t: model::Record| Record {
                    sq: t.sq,
                    content: t.content,
                })
                .collect::<Vec<_>>(),
        )),
        Err(err) => Err(ApiError::BadRequest {
            message: err.to_string(),
        }),
    }
}

#[derive(Debug, Deserialize)]
struct NewCommit {
    pub unit: Uuid,
    #[serde(rename = "recordList")]
    pub record_list: Vec<Record>,
}

#[actix_web::post("/commit")]
pub async fn add(
    pool: web::Data<DbPool>,
    new_commit: web::Json<NewCommit>,
) -> Result<web::Json<Uuid>, ApiError> {
    let mut conn = pool.get().map_err(|_| ApiError::ServerError)?;

    let new_commit = new_commit.into_inner();
    let commit_id = Uuid::new_v4();

    let result = web::block(move || {
        diesel::insert_into(commit::table)
            .values(model::Commit {
                id: commit_id,
                unit_id: new_commit.unit,
                created_at: Utc::now().naive_utc(),
            })
            .execute(&mut conn)?;
        diesel::insert_into(record::table)
            .values(
                new_commit
                    .record_list
                    .into_iter()
                    .map(|t| model::Record {
                        commit_id: commit_id,
                        sq: t.sq,
                        content: t.content,
                    })
                    .collect::<Vec<_>>(),
            )
            .execute(&mut conn)
    })
    .await
    .map_err(|_| ApiError::ServerError)?;

    match result {
        Ok(_) => Ok(web::Json(commit_id)),
        Err(err) => Err(ApiError::BadRequest {
            message: err.to_string(),
        }),
    }
}
