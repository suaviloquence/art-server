#[cfg(feature = "sqlite")]
pub type DB = sqlx::Sqlite;

pub type Pool = sqlx::Pool<DB>;
pub trait Executor<'a>: sqlx::Executor<'a, Database = DB> {}

impl<'a, T> Executor<'a> for T where T: sqlx::Executor<'a, Database = DB> {}
