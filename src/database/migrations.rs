use anyhow::Context;
use rusqlite::Connection;

pub fn migrate(connection: &mut Connection) -> anyhow::Result<()> {
    let version = database_version(connection)?;

    if version < 1 {
        build_database(connection)?;
    }

    Ok(())
}

fn database_version(conn: &Connection) -> anyhow::Result<usize> {
    //first check if we have a APP_INFO table, which contains the database version (since version 2)
    let app_info_exists = conn.query_row(
        r#"SELECT count(*) FROM sqlite_master WHERE type='table' AND name='app_info'"#,
        [],
        |row| row.get::<usize, usize>(0)
    ).context("checking for existance of `app_info` table")? == 1;

    if app_info_exists {
        //try to get version from app_info table
        conn.query_row(
            r#"SELECT value FROM app_info WHERE key='version'"#,
            [],
            |row| row.get::<usize, usize>(0)
        ).context("error reading version from app_info table")
    } else {
        //check for version 0 (empty database)
        let time_blocks_exists = conn.query_row(
            r#"SELECT count(*) FROM sqlite_master WHERE type='table' AND name='time_blocks'"#,
            [],
            |row| row.get::<usize, usize>(0)
        ).context("checking for existance of `time_blocks` table")? == 1;

        if time_blocks_exists {
            Ok(1)
        } else {
            Ok(0)
        }
    }

}

fn build_database(conn: &Connection) -> anyhow::Result<()> {
    conn.execute(
        r#"CREATE TABLE "tags" (
            "id"	INTEGER NOT NULL,
            "name"	TEXT NOT NULL UNIQUE,
            "protected"	TEXT CHECK("protected" = 'Y'),
                 PRIMARY KEY("id")
    );"#,
    [], // empty list of parameters.
    )
    .context("Failed to initailize tags table")?;
    conn.execute(
        r#"
        CREATE TABLE "time_blocks" (
            "id"	INTEGER,
            "start"	TEXT NOT NULL,
            "end"	TEXT NOT NULL,
            "running"	TEXT CHECK("running" = 'Y') UNIQUE,
                 "tag"	INTEGER,
                 FOREIGN KEY("tag") REFERENCES "tags"("id"),
                 PRIMARY KEY("id")
    );"#,
    [],
    )
    .context("Failed to initailize time_blocks table")?;

    Ok(())
}
