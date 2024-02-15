use std::any::Any;

use actix_web::{web, HttpResponse};
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql::*;
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse};
use diesel::prelude::*;
use uuid::Uuid;

use crate::model;
use crate::schema::*;

use diesel::{r2d2::ConnectionManager, PgConnection};

pub type ConnectionPool = r2d2::Pool<ConnectionManager<PgConnection>>;

pub struct Query;
pub struct Mutation;

pub type AppSchema = Schema<Query, Mutation, EmptySubscription>;

#[Object]
impl Query {
    async fn projects(
        &self,
        ctx: &Context<'_>,
        project_id: Option<Uuid>,
    ) -> Result<Vec<model::Project>> {
        let mut conn = ctx.data::<ConnectionPool>()?.get()?;

        Ok(match project_id {
            Some(id) => project::table
                .filter(project::dsl::id.eq(id))
                .load::<model::Project>(&mut conn)?,
            None => project::table.load::<model::Project>(&mut conn)?,
        })
    }
}

#[ComplexObject]
impl model::Project {
    async fn units(&self, ctx: &Context<'_>) -> Result<Vec<model::Unit>> {
        let mut conn = ctx.data::<ConnectionPool>()?.get()?;

        Ok(unit::table
            .filter(unit::dsl::project_id.eq(self.id))
            .order_by(unit::dsl::title)
            .load::<model::Unit>(&mut conn)?)
    }
}

#[ComplexObject]
impl model::Unit {
    async fn sources(&self, ctx: &Context<'_>) -> Result<Vec<model::Source>> {
        let mut conn = ctx.data::<ConnectionPool>()?.get()?;

        Ok(source::table
            .filter(source::dsl::unit_id.eq(self.id))
            .order_by(source::dsl::sq)
            .load::<model::Source>(&mut conn)?)
    }

    async fn commits(&self, ctx: &Context<'_>) -> Result<Vec<model::Commit>> {
        let mut conn = ctx.data::<ConnectionPool>()?.get()?;

        Ok(commit::table
            .filter(commit::dsl::unit_id.eq(self.id))
            .order_by(commit::dsl::created_at)
            .load::<model::Commit>(&mut conn)?)
    }

    async fn latest_commit(&self, ctx: &Context<'_>) -> Result<Option<model::Commit>> {
        let mut conn = ctx.data::<ConnectionPool>()?.get()?;

        Ok(match self.commit_id {
            Some(commit_id) => Some(
                commit::table
                    .filter(commit::dsl::id.eq(commit_id))
                    .order_by(commit::dsl::created_at)
                    .first::<model::Commit>(&mut conn)?,
            ),
            None => None,
        })
    }
}

#[ComplexObject]
impl model::Commit {
    async fn records(&self, ctx: &Context<'_>) -> Result<Vec<model::Record>> {
        let mut conn = ctx.data::<ConnectionPool>()?.get()?;

        Ok(record::table
            .filter(record::dsl::commit_id.eq(self.id))
            .order_by(record::dsl::sq)
            .load::<model::Record>(&mut conn)?)
    }
}

#[derive(InputObject)]
struct ProjectInput {
    name: String,
}

#[Object]
impl Mutation {
    async fn project_add(
        &self,
        ctx: &Context<'_>,
        project_input: ProjectInput,
    ) -> Result<model::Project> {
        let mut conn = ctx.data::<ConnectionPool>().unwrap().get().unwrap();

        let project = model::Project {
            id: Uuid::new_v4(),
            name: project_input.name,
        };

        diesel::insert_into(project::table)
            .values(&project)
            .execute(&mut conn)?;

        Ok(project)
    }
}

async fn index(schema: web::Data<AppSchema>, request: GraphQLRequest) -> GraphQLResponse {
    let query = request.into_inner();
    schema.execute(query).await.into()
}

async fn playground() -> HttpResponse {
    let config = GraphQLPlaygroundConfig::new("/graphql");

    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(playground_source(config))
}

pub fn configure_service(config: &mut web::ServiceConfig) {
    config.service(
        web::resource("/graphql")
            .route(web::get().to(playground))
            .route(web::post().to(index)),
    );
}

pub fn create_schema<T>(pool: T) -> AppSchema
where
    T: Any + Send + Sync,
{
    Schema::build(Query, Mutation, EmptySubscription)
        .data(pool)
        .finish()
}
