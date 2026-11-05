use anyhow::{Context, Result};
use chrono::Local;
use serde::Serialize;
use std::fs;

use crate::config::Settings;
use crate::db::Store;
use crate::model::{DATE_FMT, Session, TIME_FMT};
use crate::util::{ensure_private_dir, restrict_file};

const TEMPLATE: &str = include_str!("dashboard/index.html");
const STYLES: &str = include_str!("dashboard/styles.css");
const APP: &str = include_str!("dashboard/app.js");

#[derive(Serialize)]
struct ReportRecord {
    id: i64,
    date: String,
    time: String,
    kind: &'static str,
    partner: Option<String>,
    acts: Vec<String>,
    protection: bool,
    orgasms: u32,
    satisfaction: u8,
    duration: u32,
    notes: Option<String>,
}

#[derive(Serialize)]
struct Payload {
    generated_at: String,
    today: String,
    records: Vec<ReportRecord>,
}

pub fn generate(store: &Store, settings: &Settings) -> Result<std::path::PathBuf> {
    let sessions = store.all()?;
    let records: Vec<ReportRecord> = sessions.iter().map(ReportRecord::from_session).collect();
    let count = records.len();

    let now = Local::now();
    let generated_at = now.format("%Y-%m-%d %H:%M").to_string();
    let today = now.format(DATE_FMT).to_string();

    let payload = Payload {
        generated_at: generated_at.clone(),
        today: today.clone(),
        records,
    };
    let data_json = serde_json::to_string(&payload).context("序列化仪表盘数据失败")?;
    let data_json = data_json
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e");

    let html = TEMPLATE
        .replace("/*__SEXLOG_STYLES__*/", STYLES)
        .replace("/*__SEXLOG_APP__*/", APP)
        .replace("__SEXLOG_GENERATED_AT__", &generated_at)
        .replace("__SEXLOG_TODAY__", &today)
        .replace("__SEXLOG_RECORD_COUNT__", &count.to_string())
        .replace("__SEXLOG_DATA__", &data_json);

    ensure_private_dir(&settings.report_dir)?;
    let output = settings.report_dir.join("index.html");
    fs::write(&output, html).with_context(|| format!("无法写入仪表盘 {}", output.display()))?;
    restrict_file(&output)?;

    Ok(output)
}

impl ReportRecord {
    fn from_session(session: &Session) -> Self {
        Self {
            id: session.id,
            date: session.occurred_at.format(DATE_FMT).to_string(),
            time: session.occurred_at.format(TIME_FMT).to_string(),
            kind: session.kind.as_str(),
            partner: session.partner.clone(),
            acts: session.acts.clone(),
            protection: session.protection,
            orgasms: session.orgasms,
            satisfaction: session.satisfaction,
            duration: session.duration_min,
            notes: session.notes.clone(),
        }
    }
}
