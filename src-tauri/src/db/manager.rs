use rusqlite::{Connection, Result};
use std::sync::Mutex;

static DB: once_cell::sync::Lazy<Mutex<Option<Connection>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(None));

/// Initialize the SQLite database and run all migrations.
/// PRD-01 US-002 will add the full migration chain here.
pub fn initialize(path: &str) -> Result<()> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

    // Migrations will be added by PRD-01 US-002
    log::info!("Database ready at {}", path);

    let mut db = DB.lock().unwrap();
    *db = Some(conn);
    Ok(())
}

/// Get a reference to the database connection.
pub fn get_connection() -> std::sync::MutexGuard<'static, Option<Connection>> {
    DB.lock().unwrap()
}
