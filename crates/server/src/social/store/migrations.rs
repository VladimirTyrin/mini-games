use rusqlite::Connection;

use super::StoreError;

/// Schema changes in order; never edit one that has shipped, add the next. `PRAGMA user_version`
/// is the number applied, so a database is upgraded by running the ones after it at start-up.
const MIGRATIONS: &[&str] = &[include_str!("migrations/0001_social.sql")];

pub(super) fn migrate(connection: &mut Connection) -> Result<(), StoreError> {
    let applied: i64 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .map_err(StoreError::Sql)?;
    // A negative version is no version of ours either.
    let applied = usize::try_from(applied).unwrap_or(usize::MAX);
    if applied > MIGRATIONS.len() {
        // An older build than the one that wrote the database: its queries may not fit.
        return Err(StoreError::NewerThanServer {
            database: applied,
            server: MIGRATIONS.len(),
        });
    }
    for (index, migration) in MIGRATIONS.iter().enumerate().skip(applied) {
        let version = index + 1;
        let transaction = connection.transaction().map_err(StoreError::Sql)?;
        transaction
            .execute_batch(migration)
            .and_then(|()| transaction.pragma_update(None, "user_version", version as i64))
            .and_then(|()| transaction.commit())
            .map_err(|source| StoreError::Migration { version, source })?;
        tracing::info!(version, "database migrated");
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn latest_version() -> usize {
    MIGRATIONS.len()
}
