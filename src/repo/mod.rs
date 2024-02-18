use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::{r2d2::ConnectionManager, PgConnection};
use juniper::GraphQLObject;
use uuid::Uuid;

use crate::schema;

type ConnectionPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[derive(Clone)]
pub struct Repo {
    pool: ConnectionPool,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = schema::user)]
pub struct User {
    pub id: Uuid,
    pub name: String,
    pub hash: String,
    pub is_admin: bool,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = schema::project)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = schema::unit)]
pub struct Unit {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
    pub commit_id: Option<Uuid>,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = schema::commit)]
pub struct Commit {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub created_at: NaiveDateTime,
    pub editor_id: Uuid,
}

#[derive(Queryable, Selectable, Insertable, GraphQLObject)]
#[diesel(table_name = schema::source)]
pub struct Source {
    pub unit_id: Uuid,
    pub sq: i32,
    pub content: String,
    pub meta: String,
}

#[derive(Queryable, Selectable, Insertable, GraphQLObject)]
#[diesel(table_name = schema::record)]
pub struct Record {
    pub commit_id: Uuid,
    pub sq: i32,
    pub content: String,
}

impl Repo {
    pub fn new(pool: ConnectionPool) -> Self {
        Self { pool }
    }

    pub fn get_user_by_name(&self, name: String) -> Option<User> {
        let mut conn = self.pool.get().ok()?;

        schema::user::table
            .filter(schema::user::dsl::name.eq(name))
            .first::<User>(&mut conn)
            .ok()
    }

    pub fn add_user(&self, user: User) -> Option<()> {
        let mut conn = self.pool.get().ok()?;

        diesel::insert_into(schema::user::table)
            .values(&user)
            .execute(&mut conn)
            .ok()?;

        Some(())
    }

    pub fn get_project(&self) -> Option<Vec<Project>> {
        let mut conn = self.pool.get().ok()?;

        schema::project::table.load::<Project>(&mut conn).ok()
    }

    pub fn get_project_by_id(&self, id: Uuid) -> Option<Project> {
        let mut conn = self.pool.get().ok()?;

        schema::project::table
            .filter(schema::project::id.eq(id))
            .first::<Project>(&mut conn)
            .ok()
    }

    pub fn add_project(&self, project: Project) -> Option<()> {
        let mut conn = self.pool.get().ok()?;

        diesel::insert_into(schema::project::table)
            .values(&project)
            .execute(&mut conn)
            .ok()?;

        Some(())
    }

    pub fn get_unit_by_project_id(&self, project_id: Uuid) -> Option<Vec<Unit>> {
        let mut conn = self.pool.get().ok()?;

        schema::unit::table
            .filter(schema::unit::project_id.eq(project_id))
            .load::<Unit>(&mut conn)
            .ok()
    }

    pub fn get_unit_by_id(&self, id: Uuid) -> Option<Unit> {
        let mut conn = self.pool.get().ok()?;

        schema::unit::table
            .filter(schema::unit::id.eq(id))
            .first::<Unit>(&mut conn)
            .ok()
    }

    pub fn add_unit(&self, unit: Unit, source_list: Vec<Source>) -> Option<()> {
        let mut conn = self.pool.get().ok()?;

        conn.transaction(|conn| {
            diesel::insert_into(schema::unit::table)
                .values(unit)
                .execute(conn)?;

            diesel::insert_into(schema::source::table)
                .values(source_list)
                .execute(conn)
        })
        .ok()?;

        Some(())
    }

    pub fn get_source_by_unit_id(&self, unit_id: Uuid) -> Option<Vec<Source>> {
        let mut conn = self.pool.get().ok()?;

        schema::source::table
            .filter(schema::source::unit_id.eq(unit_id))
            .load::<Source>(&mut conn)
            .ok()
    }

    pub fn get_commit_by_unit_id(&self, unit_id: Uuid) -> Option<Vec<Commit>> {
        let mut conn = self.pool.get().ok()?;

        schema::commit::table
            .filter(schema::commit::unit_id.eq(unit_id))
            .load::<Commit>(&mut conn)
            .ok()
    }

    pub fn get_commit_by_id(&self, id: Uuid) -> Option<Commit> {
        let mut conn = self.pool.get().ok()?;

        schema::commit::table
            .filter(schema::commit::id.eq(id))
            .first::<Commit>(&mut conn)
            .ok()
    }

    pub fn add_commit(&self, commit: Commit, record_list: Vec<Record>) -> Option<()> {
        let mut conn = self.pool.get().ok()?;

        conn.transaction(|conn| {
            diesel::insert_into(schema::commit::table)
                .values(commit)
                .execute(conn)?;

            diesel::insert_into(schema::record::table)
                .values(record_list)
                .execute(conn)
        })
        .ok()?;

        Some(())
    }

    pub fn get_record_by_commit_id(&self, commit_id: Uuid) -> Option<Vec<Record>> {
        let mut conn = self.pool.get().ok()?;

        schema::record::table
            .filter(schema::record::commit_id.eq(commit_id))
            .load::<Record>(&mut conn)
            .ok()
    }
}
