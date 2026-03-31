use rusqlite::{Connection, Result as SqliteResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkHistoryEntry {
    pub id: i64,
    pub device: String,
    pub serial: String,
    pub block_size_kb: i32,
    pub read_speed_mbps: f64,
    pub write_speed_mbps: f64,
    pub timestamp: String,
}

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn new() -> SqliteResult<Self> {
        let db_path = Self::get_db_path();

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(&db_path)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS benchmark_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                device TEXT NOT NULL,
                serial TEXT NOT NULL,
                block_size_kb INTEGER NOT NULL,
                read_speed_mbps REAL NOT NULL,
                write_speed_mbps REAL NOT NULL,
                timestamp TEXT NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_benchmark_device_serial ON benchmark_history(device, serial)",
            [],
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn get_db_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        home.join(".local")
            .join("share")
            .join("dumbctl")
            .join("history.db")
    }

    pub fn save_benchmark(
        &self,
        device: &str,
        serial: &str,
        results: &[(i32, f64, f64)],
    ) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        for (block_kb, read_mbps, write_mbps) in results {
            conn.execute(
                "INSERT INTO benchmark_history (device, serial, block_size_kb, read_speed_mbps, write_speed_mbps, timestamp) 
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                [device, serial, &block_kb.to_string(), &read_mbps.to_string(), &write_mbps.to_string(), &timestamp],
            )?;
        }

        Ok(())
    }

    pub fn get_history(
        &self,
        device: &str,
        serial: &str,
        limit: i32,
    ) -> SqliteResult<Vec<BenchmarkHistoryEntry>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, device, serial, block_size_kb, read_speed_mbps, write_speed_mbps, timestamp 
             FROM benchmark_history 
             WHERE device = ?1 AND serial = ?2 
             ORDER BY timestamp DESC 
             LIMIT ?3"
        )?;

        let rows = stmt.query_map([device, serial, &limit.to_string()], |row| {
            Ok(BenchmarkHistoryEntry {
                id: row.get(0)?,
                device: row.get(1)?,
                serial: row.get(2)?,
                block_size_kb: row.get(3)?,
                read_speed_mbps: row.get(4)?,
                write_speed_mbps: row.get(5)?,
                timestamp: row.get(6)?,
            })
        })?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(row?);
        }

        Ok(entries)
    }

    #[allow(dead_code)]
    pub fn get_latest_for_device(
        &self,
        device: &str,
        serial: &str,
    ) -> SqliteResult<Option<BenchmarkHistoryEntry>> {
        let entries = self.get_history(device, serial, 1)?;
        Ok(entries.into_iter().next())
    }

    #[allow(dead_code)]
    pub fn get_all_devices(&self) -> SqliteResult<Vec<(String, String)>> {
        let conn = self.conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT DISTINCT device, serial FROM benchmark_history ORDER BY timestamp DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;

        let mut devices = Vec::new();
        for row in rows {
            devices.push(row?);
        }

        Ok(devices)
    }

    pub fn clear_all_history(&self) -> SqliteResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM benchmark_history", [])?;
        Ok(())
    }
}

impl Default for Database {
    fn default() -> Self {
        Self::new().expect("Failed to initialize database")
    }
}
