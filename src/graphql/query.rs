use chrono::{DateTime, Utc};
use juniper::FieldResult;
use uuid::Uuid;

use crate::repo;

use super::{Context, QueryRoot, ServiceError};

#[juniper::graphql_object(Context = Context)]
impl repo::Project {
    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn unit_list(&self, ctx: &Context) -> FieldResult<Vec<repo::Unit>> {
        ctx.repo
            .get_unit_by_project_id(self.id)
            .ok_or(ServiceError.into())
    }
}

#[juniper::graphql_object(Context = Context)]
impl repo::Unit {
    fn id(&self) -> Uuid {
        self.id
    }

    fn project_id(&self) -> Uuid {
        self.project_id
    }

    fn title(&self) -> &str {
        &self.title
    }

    fn latest_commit_id(&self) -> Option<Uuid> {
        self.commit_id
    }

    fn project(&self, ctx: &Context) -> FieldResult<repo::Project> {
        ctx.repo
            .get_project_by_id(self.project_id)
            .ok_or(ServiceError.into())
    }

    fn commit_list(&self, ctx: &Context) -> FieldResult<Vec<repo::Commit>> {
        ctx.repo
            .get_commit_by_unit_id(self.id)
            .ok_or(ServiceError.into())
    }

    fn source_list(&self, ctx: &Context) -> FieldResult<Vec<repo::Source>> {
        ctx.repo
            .get_source_by_unit_id(self.id)
            .ok_or(ServiceError.into())
    }
}

#[juniper::graphql_object(Context = Context)]
impl repo::Commit {
    fn id(&self) -> Uuid {
        self.id
    }

    fn unit_id(&self) -> Uuid {
        self.unit_id
    }

    fn created_at(&self) -> DateTime<Utc> {
        self.created_at.and_utc()
    }

    fn editor_id(&self) -> Uuid {
        self.editor_id
    }

    fn unit(&self, ctx: &Context) -> FieldResult<repo::Unit> {
        ctx.repo
            .get_unit_by_id(self.unit_id)
            .ok_or(ServiceError.into())
    }

    fn record_list(&self, ctx: &Context) -> FieldResult<Vec<repo::Record>> {
        ctx.repo
            .get_record_by_commit_id(self.id)
            .ok_or(ServiceError.into())
    }
}

#[juniper::graphql_object(Context = Context)]
impl QueryRoot {
    fn project_list(ctx: &Context) -> FieldResult<Vec<repo::Project>> {
        ctx.repo.get_project().ok_or(ServiceError.into())
    }

    fn project(ctx: &Context, id: Uuid) -> FieldResult<repo::Project> {
        ctx.repo.get_project_by_id(id).ok_or(ServiceError.into())
    }

    fn unit(ctx: &Context, id: Uuid) -> FieldResult<repo::Unit> {
        ctx.repo.get_unit_by_id(id).ok_or(ServiceError.into())
    }

    fn commit(ctx: &Context, id: Uuid) -> FieldResult<repo::Commit> {
        ctx.repo.get_commit_by_id(id).ok_or(ServiceError.into())
    }
}
