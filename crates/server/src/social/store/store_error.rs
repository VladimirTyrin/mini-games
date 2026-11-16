#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("database error")]
    Sql(#[source] rusqlite::Error),
    #[error("database migration {version} failed")]
    Migration {
        version: usize,
        #[source]
        source: rusqlite::Error,
    },
    #[error(
        "the database is at schema version {database}, this server knows {server}: it was written by a newer server"
    )]
    NewerThanServer { database: usize, server: usize },
}
