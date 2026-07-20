use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct Storage {
    conn: Arc<Mutex<Connection>>,
}

impl Storage {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;

        let storage = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        storage.initialize_schema()?;
        Ok(storage)
    }

    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;

        let storage = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        storage.initialize_schema()?;
        Ok(storage)
    }

    fn initialize_schema(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                timestamp DATETIME NOT NULL,
                source TEXT NOT NULL,
                category TEXT NOT NULL,
                event_type TEXT NOT NULL,
                message TEXT NOT NULL,
                metadata TEXT
            )",
            [],
        )?;
        Ok(())
    }
}
