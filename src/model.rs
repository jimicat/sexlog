use anyhow::{Result, bail};
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use clap::ValueEnum;
use serde::Serialize;

pub const DATE_FMT: &str = "%Y-%m-%d";
pub const TIME_FMT: &str = "%H:%M";
pub const DATETIME_FMT: &str = "%Y-%m-%d %H:%M";

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Partner,
    Solo,
    Other,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Partner => "partner",
            Self::Solo => "solo",
            Self::Other => "other",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Partner => "伴侣",
            Self::Solo => "独自",
            Self::Other => "其他",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        match value {
            "partner" => Some(Self::Partner),
            "solo" => Some(Self::Solo),
            "other" => Some(Self::Other),
            _ => None,
        }
    }

    pub fn parse_input(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "partner" | "伴侣" | "1" | "p" => Some(Self::Partner),
            "solo" | "独自" | "自慰" | "2" | "s" => Some(Self::Solo),
            "other" | "其他" | "3" | "o" => Some(Self::Other),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NewSession {
    pub occurred_at: NaiveDateTime,
    pub kind: Kind,
    pub partner: Option<String>,
    pub acts: Vec<String>,
    pub protection: bool,
    pub orgasms: u32,
    pub satisfaction: u8,
    pub duration_min: u32,
    pub notes: Option<String>,
}

impl NewSession {
    pub fn validated(mut self) -> Result<Self> {
        let date = self.occurred_at.date();
        let time = self.occurred_at.time();
        let time = time
            .with_second(0)
            .and_then(|value| value.with_nanosecond(0))
            .unwrap_or(time);
        self.occurred_at = NaiveDateTime::new(date, time);
        self.partner = clean_optional(self.partner, 64, "伴侣")?;
        self.notes = clean_optional(self.notes, 500, "备注")?;
        self.acts = normalize_acts(self.acts)?;
        if !(1..=5).contains(&self.satisfaction) {
            bail!("满意度需在 1-5 之间");
        }
        if !(1..=1440).contains(&self.duration_min) {
            bail!("时长需在 1-1440 分钟之间");
        }
        if self.orgasms > 20 {
            bail!("高潮次数不能超过 20");
        }
        Ok(self)
    }
}

#[derive(Debug, Clone)]
pub struct Session {
    pub id: i64,
    pub occurred_at: NaiveDateTime,
    pub kind: Kind,
    pub partner: Option<String>,
    pub acts: Vec<String>,
    pub protection: bool,
    pub orgasms: u32,
    pub satisfaction: u8,
    pub duration_min: u32,
    pub notes: Option<String>,
}

fn clean_optional(value: Option<String>, max_chars: usize, field: &str) -> Result<Option<String>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let value = value.trim().to_string();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > max_chars {
        bail!("{field}最多 {max_chars} 个字符");
    }
    Ok(Some(value))
}

pub fn normalize_acts(values: Vec<String>) -> Result<Vec<String>> {
    let mut acts: Vec<String> = Vec::new();
    for value in values {
        for part in value.split([',', '，', '、']) {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            if part.chars().count() > 32 {
                bail!("单个行为标签最多 32 个字符");
            }
            if !acts.iter().any(|existing| existing == part) {
                acts.push(part.to_string());
            }
        }
    }
    if acts.len() > 30 {
        bail!("行为标签最多 30 个");
    }
    Ok(acts)
}

pub fn acts_from_db(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub fn parse_date(value: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(value.trim(), DATE_FMT)
        .map_err(|_| format!("日期格式应为 YYYY-MM-DD，收到 “{value}”"))
}

pub fn parse_time(value: &str) -> Result<NaiveTime, String> {
    let trimmed = value.trim();
    let time = NaiveTime::parse_from_str(trimmed, TIME_FMT)
        .or_else(|_| NaiveTime::parse_from_str(trimmed, "%H:%M:%S"))
        .map_err(|_| format!("时间格式应为 HH:MM，收到 “{value}”"))?;
    Ok(time.with_second(0).unwrap_or(time))
}

pub fn parse_yes_no(value: &str) -> Result<bool, String> {
    match value.trim().to_lowercase().as_str() {
        "y" | "yes" | "true" | "1" | "是" | "有" => Ok(true),
        "n" | "no" | "false" | "0" | "否" | "无" => Ok(false),
        _ => Err(format!(
            "请使用 yes/no 表示是否采取保护措施，收到 “{value}”"
        )),
    }
}

pub fn parse_satisfaction(value: &str) -> Result<u8, String> {
    let parsed = value
        .trim()
        .parse::<u8>()
        .map_err(|_| format!("满意度应为 1-5 的整数，收到 “{value}”"))?;
    if (1..=5).contains(&parsed) {
        Ok(parsed)
    } else {
        Err("满意度应在 1-5 之间".to_string())
    }
}

pub fn parse_duration(value: &str) -> Result<u32, String> {
    let parsed = value
        .trim()
        .parse::<u32>()
        .map_err(|_| format!("时长应为整数分钟，收到 “{value}”"))?;
    if (1..=1440).contains(&parsed) {
        Ok(parsed)
    } else {
        Err("时长应在 1-1440 分钟之间".to_string())
    }
}

pub fn parse_orgasms(value: &str) -> Result<u32, String> {
    let parsed = value
        .trim()
        .parse::<u32>()
        .map_err(|_| format!("高潮次数应为非负整数，收到 “{value}”"))?;
    if parsed <= 20 {
        Ok(parsed)
    } else {
        Err("高潮次数不能超过 20".to_string())
    }
}

pub fn parse_port(value: &str) -> Result<u16, String> {
    let port = value
        .trim()
        .parse::<u16>()
        .map_err(|_| format!("端口应为 1-65535 的整数，收到 “{value}”"))?;
    if port == 0 {
        Err("端口不能为 0".to_string())
    } else {
        Ok(port)
    }
}
