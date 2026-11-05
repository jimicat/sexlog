use anyhow::{Context, Result};
use chrono::{Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use std::fmt;
use std::io::{self, Write};

use crate::model::{self, DATE_FMT, Kind, NewSession, Session, TIME_FMT};

#[derive(Debug)]
struct Cancelled;

impl fmt::Display for Cancelled {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("已取消")
    }
}

impl std::error::Error for Cancelled {}

pub fn collect(defaults: Option<&Session>) -> Result<Option<NewSession>> {
    match collect_inner(defaults) {
        Ok(session) => Ok(Some(session)),
        Err(error) if error.downcast_ref::<Cancelled>().is_some() => {
            println!("已取消，未保存。");
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

fn collect_inner(defaults: Option<&Session>) -> Result<NewSession> {
    println!();
    println!("交互式录入");
    println!("直接回车采用方括号中的默认值，Ctrl+C 退出。");
    println!();

    let now = Local::now().naive_local();
    let date = ask_date(now.date())?;
    let time = ask_time(now.time())?;

    let default_kind = defaults
        .map(|session| session.kind)
        .unwrap_or(Kind::Partner);
    let kind = ask_kind(default_kind)?;

    let default_partner = defaults.and_then(|session| session.partner.as_deref());
    let partner = ask_partner(default_partner)?;

    let default_acts = defaults
        .map(|session| session.acts.join(", "))
        .unwrap_or_default();
    let acts = ask_acts(&default_acts)?;

    let default_protection = defaults.map(|session| session.protection).unwrap_or(true);
    let protection = ask_yes_no("保护措施", default_protection)?;

    let default_orgasms = defaults.map(|session| session.orgasms).unwrap_or(1);
    let orgasms = ask_parsed("高潮次数", default_orgasms, model::parse_orgasms)?;

    let default_satisfaction = defaults.map(|session| session.satisfaction).unwrap_or(3);
    let satisfaction = ask_parsed(
        "满意度 1-5",
        default_satisfaction,
        model::parse_satisfaction,
    )?;

    let default_duration = defaults.map(|session| session.duration_min).unwrap_or(30);
    let duration_min = ask_parsed("时长（分钟）", default_duration, model::parse_duration)?;

    let notes = ask_notes(defaults.and_then(|session| session.notes.as_deref()))?;

    let session = NewSession {
        occurred_at: NaiveDateTime::new(date, time),
        kind,
        partner,
        acts,
        protection,
        orgasms,
        satisfaction,
        duration_min,
        notes,
    }
    .validated()?;

    print_summary(&session);

    if !ask_yes_no("保存这条记录？", true)? {
        return Err(Cancelled.into());
    }

    Ok(session)
}

fn ask_date(default: NaiveDate) -> Result<NaiveDate> {
    loop {
        let default_text = default.format(DATE_FMT).to_string();
        let input = ask("日期 (YYYY-MM-DD)", Some(&default_text))?;
        match model::parse_date(&input) {
            Ok(date) => return Ok(date),
            Err(error) => eprintln!("{error}"),
        }
    }
}

fn ask_time(default: NaiveTime) -> Result<NaiveTime> {
    let default = default.with_second(0).unwrap_or(default);
    loop {
        let default_text = default.format(TIME_FMT).to_string();
        let input = ask("时间 (HH:MM)", Some(&default_text))?;
        match model::parse_time(&input) {
            Ok(time) => return Ok(time),
            Err(error) => eprintln!("{error}"),
        }
    }
}

fn ask_kind(default: Kind) -> Result<Kind> {
    let default_text = match default {
        Kind::Partner => "1",
        Kind::Solo => "2",
        Kind::Other => "3",
    };
    loop {
        let input = ask("类型 (1 伴侣 / 2 独自 / 3 其他)", Some(default_text))?;
        if let Some(kind) = Kind::parse_input(&input) {
            return Ok(kind);
        }
        eprintln!("请输入 1、2、3，或 partner / solo / other");
    }
}

fn ask_partner(default: Option<&str>) -> Result<Option<String>> {
    loop {
        let input = ask("伴侣（可留空）", default)?;
        let input = input.trim();
        if input.chars().count() > 64 {
            eprintln!("伴侣最多 64 个字符");
            continue;
        }
        return Ok(if input.is_empty() {
            None
        } else {
            Some(input.to_string())
        });
    }
}

fn ask_acts(default: &str) -> Result<Vec<String>> {
    loop {
        let input = ask("行为标签（逗号分隔，可留空）", Some(default))?;
        match model::normalize_acts(vec![input]) {
            Ok(acts) => return Ok(acts),
            Err(error) => eprintln!("{error}"),
        }
    }
}

fn ask_notes(default: Option<&str>) -> Result<Option<String>> {
    loop {
        let input = ask("备注（可留空）", default)?;
        let input = input.trim();
        if input.chars().count() > 500 {
            eprintln!("备注最多 500 个字符");
            continue;
        }
        return Ok(if input.is_empty() {
            None
        } else {
            Some(input.to_string())
        });
    }
}

fn ask_yes_no(label: &str, default: bool) -> Result<bool> {
    let hint = if default { "Y/n" } else { "y/N" };
    loop {
        let input = read_line(&format!("{label} [{hint}]: "))?;
        if input.is_empty() {
            return Ok(default);
        }
        match model::parse_yes_no(&input) {
            Ok(value) => return Ok(value),
            Err(_) => eprintln!("请输入 y 或 n"),
        }
    }
}

fn ask_parsed<T>(label: &str, default: T, parser: impl Fn(&str) -> Result<T, String>) -> Result<T>
where
    T: fmt::Display,
{
    loop {
        let default_text = default.to_string();
        let input = ask(label, Some(&default_text))?;
        match parser(&input) {
            Ok(value) => return Ok(value),
            Err(error) => eprintln!("{error}"),
        }
    }
}

fn ask(label: &str, default: Option<&str>) -> Result<String> {
    let prompt = match default {
        Some(value) if !value.is_empty() => format!("{label} [{value}]: "),
        _ => format!("{label}: "),
    };
    let input = read_line(&prompt)?;
    if input.is_empty() {
        Ok(default.unwrap_or_default().to_string())
    } else {
        Ok(input)
    }
}

fn read_line(prompt: &str) -> Result<String> {
    print!("{prompt}");
    io::stdout().flush().context("无法输出提示信息")?;
    let mut buffer = String::new();
    let read = io::stdin().read_line(&mut buffer).context("读取输入失败")?;
    if read == 0 {
        println!();
        return Err(Cancelled.into());
    }
    Ok(buffer.trim().to_string())
}

fn print_summary(session: &NewSession) {
    let partner = session.partner.as_deref().unwrap_or("—");
    let acts = if session.acts.is_empty() {
        "—".to_string()
    } else {
        session.acts.join(" · ")
    };
    let notes = session.notes.as_deref().unwrap_or("—");

    println!();
    println!("──────── 确认记录 ────────");
    println!("时间      {}", session.occurred_at.format("%Y-%m-%d %H:%M"));
    println!("类型      {}", session.kind.label());
    println!("伴侣      {partner}");
    println!("行为      {acts}");
    println!("措施      {}", if session.protection { "是" } else { "否" });
    println!("高潮      {} 次", session.orgasms);
    println!("满意度    {} / 5", session.satisfaction);
    println!("时长      {} 分钟", session.duration_min);
    println!("备注      {notes}");
    println!("──────────────────────────");
    println!();
}
