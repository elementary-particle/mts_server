use chrono::NaiveDateTime;
use diesel::{Associations, Insertable, Queryable, Selectable};
use uuid::Uuid;

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::project)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Project {
    pub id: Uuid,
    pub name: String,
}

#[derive(Queryable, Selectable, Insertable, Associations)]
#[diesel(table_name = crate::schema::unit)]
#[diesel(belongs_to(Project))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Unit {
    pub id: Uuid,
    pub project_id: Uuid,
    pub title: String,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::source)]
#[diesel(belongs_to(Unit))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Source {
    pub unit_id: Uuid,
    pub sq: i32,
    pub content: String,
    pub meta: String,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::commit)]
#[diesel(belongs_to(Unit))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Commit {
    pub id: Uuid,
    pub unit_id: Uuid,
    pub created_at: NaiveDateTime,
}

#[derive(Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::record)]
#[diesel(belongs_to(Commit))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Record {
    pub commit_id: Uuid,
    pub sq: i32,
    pub content: String,
}
