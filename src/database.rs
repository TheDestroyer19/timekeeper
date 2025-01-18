use anyhow::{anyhow, Context};
use chrono::{DateTime, Duration, Local};
use rusqlite::Connection;
use tracing::info;

use crate::APP_NAME;

mod migrations;

/// A block of time
#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct Block {
    id: usize,
    pub start: DateTime<Local>,
    pub end: DateTime<Local>,
    pub tag: Option<Tag>,
    pub running: bool,
}

impl Block {
    pub fn duration(&self) -> Duration {
        self.end - self.start
    }

    pub fn id(&self) -> usize {
        self.id
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct Tag {
    id: usize,
    pub name: String,
}
impl PartialEq for Tag {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn new() -> Result<Self, anyhow::Error> {
        let mut conn = new_disk_connection().or_else(|e| {
            tracing::warn!("Failed to open or create database on disk. Records will not be persisted. Error is {:#}", e);
            new_in_memory_connection()
        })?;

        migrations::migrate(&mut conn)?;

        Ok(Self { conn })
    }

    pub fn stopwatch(&self) -> StopWatch<'_> {
        StopWatch {
            conn: &self.conn,
            now: Local::now(),
        }
    }

    pub fn blocks(&self) -> Blocks<'_> {
        Blocks { conn: &self.conn }
    }

    pub fn tags(&self) -> Tags<'_> {
        Tags { conn: &self.conn }
    }
}

pub struct StopWatch<'a> {
    conn: &'a Connection,
    now: DateTime<Local>,
}

impl StopWatch<'_> {
    /// Start the stopwatch
    pub fn start(&self, tag: Option<Tag>) -> Result<(), anyhow::Error> {
        let block = Block {
            id: 0,
            start: self.now,
            end: self.now,
            tag,
            running: true,
        };

        let tag = block.tag.as_ref().map(|t| t.id);
        let running = if block.running { Some("Y") } else { None };

        self.conn
            .execute(
                "
            INSERT INTO time_blocks (start, end, tag, running)
            VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![block.start, block.end, tag, running],
            )
            .map(|_| ())
            .context("Trying to insert block into database")?;

        info!("Started stopwatch at {:?}", self.now);
        Ok(())
    }

    /// Stops any running blocks
    pub fn stop(&self) -> Result<(), anyhow::Error> {
        self.conn
            .execute(
                "UPDATE time_blocks SET end = ?1, running = ?2 WHERE running = ?3",
                rusqlite::params![self.now, Option::<&str>::None, "Y"],
            )
            .map(|_| ())
            .context("Trying to stop running blocks")?;
        info!("Stopped stopwatch at {:?}", self.now);
        Ok(())
    }

    /// Update end times on running blocks
    pub fn update(&self) -> Result<(), anyhow::Error> {
        self.conn
            .execute(
                "UPDATE time_blocks SET end = ?1 WHERE running = ?2",
                rusqlite::params![self.now, "Y"],
            )
            .map(|_| ())
            .context("Trying to stop running blocks")
    }
}

pub struct Blocks<'a> {
    conn: &'a Connection,
}

impl Blocks<'_> {
    /// Converts a rustqlite row into a block
    fn to_blocks(row: &rusqlite::Row<'_>) -> Result<Block, rusqlite::Error> {
        let running: Option<String> = row.get(3)?;
        let running = running.filter(|s| s == "Y").is_some();
        let id: Option<usize> = row.get(4)?;
        let name: Option<String> = row.get(5)?;
        let tag = id.map(|id| Tag {
            id,
            name: name.expect("tags.name should not be null when tags.id is not null"),
        });
        Ok(Block {
            id: row.get(0)?,
            start: row.get(1)?,
            end: row.get(2)?,
            tag,
            running,
        })
    }

    pub fn update_tag(&self, block: Block) -> Result<(), anyhow::Error> {
        let tag = block.tag.map(|t| t.id);
        self.conn
            .execute(
                "UPDATE time_blocks
            SET tag = ?2
            WHERE id = ?1",
                rusqlite::params![block.id, tag],
            )
            .map(|_| ())
            .context("Trying to update a block")
    }

    pub fn delete(&self, block: Block) -> Result<(), anyhow::Error> {
        self.conn
            .execute("DELETE FROM time_blocks WHERE id = ?1", [block.id])
            .map(|_| ())
            .context("Trying to delete block from database")
    }

    pub fn current(&self) -> Result<Option<Block>, anyhow::Error> {
        let current = self.conn.query_row(
            "
                SELECT 
                    block.id, start, end, running, tag.id, tag.name 
                FROM time_blocks block 
                LEFT JOIN tags tag ON block.tag = tag.id
                WHERE running is 'Y'",
            [],
            Self::to_blocks,
        );

        match current {
            Ok(block) => Ok(Some(block)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            err => err.map(|_| None).context("Trying to get current block"),
        }
    }

    pub fn in_range(
        &self,
        before: DateTime<Local>,
        after: DateTime<Local>,
    ) -> Result<Vec<Block>, anyhow::Error> {
        self.conn
            .prepare(
                "
                SELECT
                    block.id, start, end, running, tag.id, tag.name
                FROM time_blocks block
                LEFT JOIN tags tag ON block.tag = tag.id
                WHERE JulianDay(start) > JulianDay(?1) 
                AND JulianDay(start) < JulianDay(?2)",
            )
            .context("Preparing to get all blocks")?
            .query_map([before, after], Self::to_blocks)
            .context("Trying to get all blocks")?
            .map(|r| r.context("Trying to map row to Block struct"))
            .collect()
    }
}

pub struct Tags<'a> {
    conn: &'a Connection
}

impl Tags<'_> {
    pub fn all(&self) -> Result<Vec<Tag>, anyhow::Error> {
        self.conn
        .prepare(
            "
            SELECT
            id, name
            FROM tags
            WHERE to_delete != 'Y'",
        )
        .context("Preparing to get all tags")?
        .query_map([], |row| {
            Ok(Tag {
                id: row.get(0)?,
               name: row.get(1)?,
            })
        })
        .context("Trying to get all tags")?
        .map(|r| r.context("Trying to map row to Tag struct"))
        .collect()
    }

    // pub fn create(&self, name: &str) -> anyhow::Result<()> {
    //     todo!()
    // }
    //
    // pub fn rename(&self, tag: Tag, new_name: &str) -> anyhow::Result<()> {
    //     todo!()
    // }
    //
    // pub fn delete(&self, tag: Tag) -> anyhow::Result<()> {
    //     todo!()
    // }
    //
    // /// Remove tags that have been marked for deletion and are no longer found in tags
    // pub fn maintain(&self) -> anyhow::Result<()> {
    //     todo!()
    // }
}

fn new_disk_connection() -> Result<Connection, anyhow::Error> {
    //get database dierectory
    let proj_dirs = directories_next::ProjectDirs::from("", "", APP_NAME)
        .ok_or(anyhow!("Saving disabled: Failed to find path to data_dir"))?;
    let data_dir = proj_dirs.data_dir().to_path_buf();

    std::fs::create_dir_all(&data_dir).with_context(|| {
        format!(
            "Saving disabled: Failed to create app path at {}",
            data_dir.display()
        )
    })?;
    let path = data_dir.join("database.sqlite");

    //open the database
    Connection::open(&path)
        .with_context(|| format!("Saving disabled: Failed to open {}", path.display()))
}

fn new_in_memory_connection() -> Result<Connection, anyhow::Error> {
    Err(anyhow!("TODO - implement in memory fallback"))
}
