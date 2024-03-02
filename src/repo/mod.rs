use chrono::NaiveDateTime;
use diesel::prelude::*;
use diesel::result::DatabaseErrorKind;
use diesel::{r2d2::ConnectionManager, PgConnection};
use juniper::GraphQLObject;
use uuid::Uuid;

use crate::schema;

type ConnectionPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[derive(Clone)]
pub struct Repo {
    pool: ConnectionPool,
}

#[derive(Debug)]
pub enum Error {
    NotFound,
    NotUnique {
        column_name: Option<String>,
    },
    ForeignKeyViolation {
        column_name: Option<String>,
    },
    ConstraintViolation {
        column_name: Option<String>,
        constraint_name: Option<String>,
    },
    DataError,
    ConnectionError(r2d2::Error),
    DieselError(diesel::result::Error),
}

impl From<diesel::result::Error> for Error {
    fn from(error: diesel::result::Error) -> Self {
        use diesel::result::Error::*;
        match error {
            DatabaseError(kind, ref info) => match kind {
                DatabaseErrorKind::UniqueViolation => Self::NotUnique {
                    column_name: info.column_name().map(String::from),
                },
                DatabaseErrorKind::ForeignKeyViolation => Self::ForeignKeyViolation {
                    column_name: info.column_name().map(String::from),
                },
                DatabaseErrorKind::CheckViolation | DatabaseErrorKind::NotNullViolation => {
                    Self::ConstraintViolation {
                        column_name: info.column_name().map(String::from),
                        constraint_name: info.constraint_name().map(String::from),
                    }
                }
                _ => Self::DieselError(error),
            },
            NotFound => Self::NotFound,
            DeserializationError(_) | SerializationError(_) => Self::DataError,
            _ => Self::DieselError(error),
        }
    }
}

impl From<r2d2::Error> for Error {
    fn from(error: r2d2::Error) -> Self {
        Self::ConnectionError(error)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotFound => write!(f, "The specified entity could not be found"),
            Error::NotUnique { column_name } => write!(
                f,
                "Column {} already exists",
                column_name.clone().unwrap_or("<?>".to_string())
            ),
            Error::ForeignKeyViolation { column_name } => {
                write!(
                    f,
                    "Foreign key {} is violated",
                    column_name.clone().unwrap_or("<?>".to_string())
                )
            }
            Error::ConstraintViolation {
                column_name,
                constraint_name,
            } => write!(
                f,
                "Constraint {} on {} is violated",
                constraint_name.clone().unwrap_or("<?>".to_string()),
                column_name.clone().unwrap_or("<?>".to_string())
            ),
            Error::DataError => write!(f, "Data value is invalid"),
            Error::ConnectionError(_) => write!(f, "Failed to connect to database"),
            Error::DieselError(_) => write!(f, "Database operation error"),
        }
    }
}

impl std::error::Error for Error {}

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

    pub fn get_user_by_name(&self, name: String) -> Result<User, Error> {
        let mut conn = self.pool.get()?;

        schema::user::table
            .filter(schema::user::dsl::name.eq(name))
            .first::<User>(&mut conn)
            .map_err(Error::from)
    }

    pub fn get_user_by_id(&self, id: Uuid) -> Result<User, Error> {
        let mut conn = self.pool.get()?;

        schema::user::table
            .filter(schema::user::id.eq(id))
            .first::<User>(&mut conn)
            .map_err(Error::from)
    }

    pub fn add_user(&self, user: User) -> Result<(), Error> {
        let mut conn = self.pool.get()?;

        diesel::insert_into(schema::user::table)
            .values(&user)
            .execute(&mut conn)?;

        Ok(())
    }

    pub fn get_project(&self) -> Result<Vec<Project>, Error> {
        let mut conn = self.pool.get()?;

        schema::project::table
            .load::<Project>(&mut conn)
            .map_err(Error::from)
    }

    pub fn get_project_by_id(&self, id: Uuid) -> Result<Project, Error> {
        let mut conn = self.pool.get()?;

        schema::project::table
            .filter(schema::project::id.eq(id))
            .first::<Project>(&mut conn)
            .map_err(Error::from)
    }

    pub fn add_project(&self, project: Project) -> Result<(), Error> {
        let mut conn = self.pool.get()?;

        diesel::insert_into(schema::project::table)
            .values(&project)
            .execute(&mut conn)?;

        Ok(())
    }

    pub fn get_unit_by_project_id(&self, project_id: Uuid) -> Result<Vec<Unit>, Error> {
        let mut conn = self.pool.get()?;

        schema::unit::table
            .filter(schema::unit::project_id.eq(project_id))
            .order_by(schema::unit::title)
            .load::<Unit>(&mut conn)
            .map_err(Error::from)
    }

    pub fn get_unit_by_id(&self, id: Uuid) -> Result<Unit, Error> {
        let mut conn = self.pool.get()?;

        schema::unit::table
            .filter(schema::unit::id.eq(id))
            .first::<Unit>(&mut conn)
            .map_err(Error::from)
    }

    pub fn add_unit(&self, unit: Unit, source_list: Vec<Source>) -> Result<(), Error> {
        let mut conn = self.pool.get()?;

        conn.transaction(|conn| {
            diesel::insert_into(schema::unit::table)
                .values(unit)
                .execute(conn)?;

            diesel::insert_into(schema::source::table)
                .values(source_list)
                .execute(conn)
        })?;

        Ok(())
    }

    pub fn get_source_by_unit_id(&self, unit_id: Uuid) -> Result<Vec<Source>, Error> {
        let mut conn = self.pool.get()?;

        schema::source::table
            .filter(schema::source::unit_id.eq(unit_id))
            .order_by(schema::source::sq)
            .load::<Source>(&mut conn)
            .map_err(Error::from)
    }

    pub fn get_commit_by_unit_id(&self, unit_id: Uuid) -> Result<Vec<Commit>, Error> {
        let mut conn = self.pool.get()?;

        schema::commit::table
            .filter(schema::commit::unit_id.eq(unit_id))
            .order_by(schema::commit::created_at)
            .load::<Commit>(&mut conn)
            .map_err(Error::from)
    }

    pub fn get_commit_by_id(&self, id: Uuid) -> Result<Commit, Error> {
        let mut conn = self.pool.get()?;

        schema::commit::table
            .filter(schema::commit::id.eq(id))
            .first::<Commit>(&mut conn)
            .map_err(Error::from)
    }

    pub fn add_commit(&self, commit: Commit, record_list: Vec<Record>) -> Result<(), Error> {
        let mut conn = self.pool.get()?;

        conn.transaction(|conn| {
            diesel::insert_into(schema::commit::table)
                .values(commit)
                .execute(conn)?;

            diesel::insert_into(schema::record::table)
                .values(record_list)
                .execute(conn)
        })?;

        Ok(())
    }

    pub fn get_record_by_commit_id(&self, commit_id: Uuid) -> Result<Vec<Record>, Error> {
        let mut conn = self.pool.get()?;

        schema::record::table
            .filter(schema::record::commit_id.eq(commit_id))
            .order_by(schema::record::sq)
            .load::<Record>(&mut conn)
            .map_err(Error::from)
    }
}
