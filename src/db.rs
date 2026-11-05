use anyhow::{Context, Result, bail};
use chrono::{NaiveDateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Row, params};
use std::path::Path;

use crate::model::{DATETIME_FMT, Kind, NewSession, Session, acts_from_db};
use crate::util::{ensure_private_dir, restrict_file};

const SCHEMA_VERSION: i64 = 1;

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            ensure_private_dir(parent)?;
        }

        let mut conn =
            Connection::open(path).with_context(|| format!("无法打开数据库 {}", path.display()))?;
        restrict_file(path)?;

        conn.execute_batch(
            "PRAGMA busy_timeout = 5000;
             PRAGMA secure_delete = ON;",
        )
        .context("无法初始化数据库连接")?;

        migrate(&mut conn)?;

        Ok(Self { conn })
    }

    pub fn insert(&self, session: &NewSession) -> Result<i64> {
        let occurred_at = session.occurred_at.format(DATETIME_FMT).to_string();
        let acts = session.acts.join(",");
        let created_at = Utc::now().to_rfc3339();
        self.conn
            .execute(
                "INSERT INTO sessions
                    (occurred_at, kind, partner, acts, protection, orgasms,
                     satisfaction, duration_min, notes, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    occurred_at,
                    session.kind.as_str(),
                    session.partner,
                    acts,
                    session.protection as i64,
                    session.orgasms,
                    session.satisfaction,
                    session.duration_min,
                    session.notes,
                    created_at,
                ],
            )
            .context("写入记录失败")?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn all(&self) -> Result<Vec<Session>> {
        let mut statement = self
            .conn
            .prepare(
                "SELECT id, occurred_at, kind, partner, acts, protection, orgasms,
                        satisfaction, duration_min, notes
                 FROM sessions
                 ORDER BY occurred_at ASC, id ASC",
            )
            .context("读取记录失败")?;
        let rows = statement
            .query_map([], read_session)
            .context("读取记录失败")?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("读取记录失败")
    }

    pub fn recent(&self, limit: u32) -> Result<Vec<Session>> {
        let mut statement = self
            .conn
            .prepare(
                "SELECT id, occurred_at, kind, partner, acts, protection, orgasms,
                        satisfaction, duration_min, notes
                 FROM sessions
                 ORDER BY occurred_at DESC, id DESC
                 LIMIT ?1",
            )
            .context("读取记录失败")?;
        let rows = statement
            .query_map([i64::from(limit)], read_session)
            .context("读取记录失败")?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .context("读取记录失败")
    }

    pub fn latest(&self) -> Result<Option<Session>> {
        let mut statement = self
            .conn
            .prepare(
                "SELECT id, occurred_at, kind, partner, acts, protection, orgasms,
                        satisfaction, duration_min, notes
                 FROM sessions
                 ORDER BY occurred_at DESC, id DESC
                 LIMIT 1",
            )
            .context("读取记录失败")?;
        statement
            .query_row([], read_session)
            .optional()
            .context("读取记录失败")
    }

    pub fn count(&self) -> Result<u64> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM sessions", [], |row| row.get(0))
            .context("读取记录数量失败")?;
        Ok(count.max(0) as u64)
    }
}

fn migrate(conn: &mut Connection) -> Result<()> {
    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .context("读取数据库版本失败")?;

    if version > SCHEMA_VERSION {
        bail!("数据库版本 {version} 高于当前程序支持的版本 {SCHEMA_VERSION}，请升级 sexlog");
    }

    if version < 1 {
        let transaction = conn.transaction().context("创建数据库事务失败")?;
        transaction
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS sessions (
                    id            INTEGER PRIMARY KEY AUTOINCREMENT,
                    occurred_at   TEXT NOT NULL,
                    kind          TEXT NOT NULL CHECK (kind IN ('partner', 'solo', 'other')),
                    partner       TEXT,
                    acts          TEXT NOT NULL DEFAULT '',
                    protection    INTEGER NOT NULL CHECK (protection IN (0, 1)),
                    orgasms       INTEGER NOT NULL DEFAULT 1 CHECK (orgasms BETWEEN 0 AND 20),
                    satisfaction  INTEGER NOT NULL CHECK (satisfaction BETWEEN 1 AND 5),
                    duration_min  INTEGER NOT NULL CHECK (duration_min BETWEEN 1 AND 1440),
                    notes         TEXT,
                    created_at    TEXT NOT NULL
                 );
                 CREATE INDEX IF NOT EXISTS idx_sessions_occurred_at
                    ON sessions (occurred_at);",
            )
            .context("创建数据表失败")?;
        transaction
            .pragma_update(None, "user_version", SCHEMA_VERSION)
            .context("写入数据库版本失败")?;
        transaction.commit().context("提交数据库事务失败")?;
    }

    Ok(())
}

fn read_session(row: &Row<'_>) -> rusqlite::Result<Session> {
    let occurred_at_raw: String = row.get("occurred_at")?;
    let occurred_at =
        NaiveDateTime::parse_from_str(&occurred_at_raw, DATETIME_FMT).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                1,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?;

    let kind_raw: String = row.get("kind")?;
    let kind = Kind::from_db(&kind_raw).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("未知类型 {kind_raw}"),
            )),
        )
    })?;

    let acts_raw: String = row.get("acts")?;
    let protection_raw: i64 = row.get("protection")?;
    let orgasms_raw: i64 = row.get("orgasms")?;
    let satisfaction_raw: i64 = row.get("satisfaction")?;
    let duration_raw: i64 = row.get("duration_min")?;

    Ok(Session {
        id: row.get("id")?,
        occurred_at,
        kind,
        partner: row.get("partner")?,
        acts: acts_from_db(&acts_raw),
        protection: protection_raw != 0,
        orgasms: orgasms_raw.max(0) as u32,
        satisfaction: satisfaction_raw.clamp(1, 5) as u8,
        duration_min: duration_raw.max(0) as u32,
        notes: row.get("notes")?,
    })
}
