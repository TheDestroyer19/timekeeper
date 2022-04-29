use chrono::{DateTime, Local};
use rusqlite::Connection;

use crate::APP_NAME;

/// A block of time
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Block {
    id: usize,
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
}

#[derive(Default)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct StopWatch {
    current: Option<Block>,
    #[serde(skip)]
    database: Option<Connection>,
}

impl StopWatch {
    pub fn init_database(&mut self) {
        //get database dierectory
        let path = if let Some(proj_dirs) = directories_next::ProjectDirs::from("", "", APP_NAME) {
            let data_dir = proj_dirs.data_dir().to_path_buf();
            if let Err(err) = std::fs::create_dir_all(&data_dir) {
                tracing::warn!(
                    "Saving disabled: Failed to create app path at {:?}: {}",
                    data_dir,
                    err
                );
                None
            } else {
                Some(data_dir.join("database.sqlite"))
            }
        } else {
            tracing::warn!("Saving disabled: Failed to find path to data_dir.");
            None
        };

        if let Some(path) = path {
            if let Ok(conn) = Connection::open(path) {
                if Err(rusqlite::Error::QueryReturnedNoRows) == conn.query_row(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name='time_blocks';", 
                    [], 
                    |row| row.get::<usize, String>(0)
                ) {
                    conn.execute(
                        "CREATE TABLE time_blocks (
                            id    INTEGER PRIMARY KEY,
                            start TEXT NOT NULL,
                            end   TEXT NOT NULL
                        )",
                        [], // empty list of parameters.
                    ).unwrap();
                }
                self.database = Some(conn);
            } else {
                panic!("Couldn't open database on disk. Maybe implement in memory db fallback");
            }
        } else {
            panic!("Couldn't open database on disk. Maybe implement in memory db fallback");
        }
    }

    pub fn start(&mut self) {
        if self.current.is_none() {
            self.current = Some(Block {
                id: 0,
                start: Local::now(),
                end: Local::now(),
            })
        } else {
            tracing::warn!("Called start on an already running stopwatch");
        }
    }

    pub fn stop(&mut self) {
        if let Some(mut block) = self.current.take() {
            block.end = Local::now();
            self.insert_block(block);
        } else {
            tracing::warn!("Called stop on an already stopped stopwatch");
        }
    }

    pub fn current(&self) -> Option<&Block> {
        self.current.as_ref()
    }

    pub fn delete_block(&mut self, block: Block) {
        let database = self.database.as_ref().expect("Database connection has been initialized");
        database.execute("DELETE FROM time_blocks WHERE id = ?1", [block.id]).unwrap();
    }

    pub fn all_blocks(&self) -> Vec<Block> {
        let database = self.database.as_ref().expect("Database connection has been initialized");
        let mut stmt = database.prepare("SELECT id, start, end FROM time_blocks").unwrap();
        stmt.query_map([], |row| Ok(Block {
            id: row.get(0)?,
            start: row.get(1)?,
            end: row.get(2)?,
        })).unwrap()
        .map(|b| b.unwrap())
        .collect()
    }

    fn insert_block(&self, block: Block) {
        let database = self.database.as_ref().expect("Database connection has been initialized");
        database.execute(
            "INSERT INTO time_blocks (start, end) VALUES (?1, ?2)",
             [block.start, block.end]
        ).unwrap();
    }
}