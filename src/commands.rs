use anyhow::Result;
use chrono::{Local, Timelike};

use crate::cli::{AddArgs, ConfigArgs, ListArgs, ServeArgs, TuiArgs};
use crate::config::Settings;
use crate::db::Store;
use crate::interactive;
use crate::model::{Kind, NewSession};
use crate::report;
use crate::server;
use crate::tui;
use crate::util::format_size;

pub fn add(args: AddArgs, settings: &Settings) -> Result<()> {
    let store = Store::open(&settings.db_file)?;
    let now = Local::now().naive_local();
    let date = args.date.unwrap_or(now.date());
    let time = args
        .time
        .unwrap_or_else(|| now.time().with_second(0).unwrap_or(now.time()));

    let session = NewSession {
        occurred_at: chrono::NaiveDateTime::new(date, time),
        kind: args.kind,
        partner: args.partner,
        acts: args.acts,
        protection: args.protection,
        orgasms: args.orgasms,
        satisfaction: args.satisfaction,
        duration_min: args.duration,
        notes: args.notes,
    }
    .validated()?;

    let id = store.insert(&session)?;
    println!(
        "已记录 #{id} · {} · {}",
        session.occurred_at.format("%Y-%m-%d %H:%M"),
        describe_identity(session.kind, session.partner.as_deref())
    );
    println!(
        "  {} · 满意度 {}/5 · {} 分钟",
        describe_details(&session.acts, session.protection, session.orgasms),
        session.satisfaction,
        session.duration_min
    );
    Ok(())
}

pub fn log(settings: &Settings) -> Result<()> {
    let store = Store::open(&settings.db_file)?;
    let latest = store.latest()?;
    if let Some(session) = interactive::collect(latest.as_ref())? {
        let id = store.insert(&session)?;
        println!(
            "已记录 #{id} · {} · {}",
            session.occurred_at.format("%Y-%m-%d %H:%M"),
            describe_identity(session.kind, session.partner.as_deref())
        );
    }
    Ok(())
}

pub fn list(args: ListArgs, settings: &Settings) -> Result<()> {
    let store = Store::open(&settings.db_file)?;
    let sessions = store.recent(args.limit)?;
    if sessions.is_empty() {
        println!("还没有记录。运行 sexlog log 或 sexlog add 开始记录。");
        return Ok(());
    }

    println!("最近 {} 条记录", sessions.len());
    println!();
    for session in sessions {
        println!(
            "#{:<4} {}  {}",
            session.id,
            session.occurred_at.format("%Y-%m-%d %H:%M"),
            describe_identity(session.kind, session.partner.as_deref())
        );
        println!(
            "      措施 {} · 高潮 {} · 满意度 {}/5 · {} 分钟",
            if session.protection { "是" } else { "否" },
            session.orgasms,
            session.satisfaction,
            session.duration_min
        );
        if !session.acts.is_empty() {
            println!("      行为  {}", session.acts.join(" · "));
        }
        if let Some(notes) = &session.notes {
            println!("      备注  {notes}");
        }
        println!();
    }
    Ok(())
}

pub fn tui(args: TuiArgs, settings: &Settings) -> Result<()> {
    let store = Store::open(&settings.db_file)?;
    tui::run(store.all()?, args)
}

pub fn report(settings: &Settings) -> Result<()> {
    let store = Store::open(&settings.db_file)?;
    let count = store.count()?;
    let path = report::generate(&store, settings)?;
    println!("仪表盘已生成: {}", path.display());
    println!("记录数: {count}");
    println!("本地预览: sexlog serve");
    Ok(())
}

pub fn serve(args: ServeArgs, settings: &Settings) -> Result<()> {
    let store = Store::open(&settings.db_file)?;
    let path = report::generate(&store, settings)?;
    println!("仪表盘已刷新: {}", path.display());

    let host = args.host.unwrap_or_else(|| settings.host.clone());
    let port = args.port.unwrap_or(settings.port);
    server::serve(&settings.report_dir, &host, port, args.open)
}

pub fn config(args: ConfigArgs, settings: &Settings) -> Result<()> {
    if args.init {
        if settings.init_config()? {
            println!("已写入默认配置: {}", settings.config_file.display());
        } else {
            println!("配置文件已存在，未覆盖: {}", settings.config_file.display());
        }
        println!();
    }

    let config_state = if settings.config_file.exists() {
        "已存在"
    } else {
        "未创建，使用默认配置"
    };
    println!(
        "配置文件  {} ({config_state})",
        settings.config_file.display()
    );
    print_db_info(settings)?;

    let report_file = settings.report_dir.join("index.html");
    let report_state = if report_file.exists() {
        "已生成"
    } else {
        "尚未生成，运行 sexlog report"
    };
    println!("仪表盘    {} ({report_state})", report_file.display());
    println!("服务      http://{}:{}", settings.host, settings.port);
    Ok(())
}

fn print_db_info(settings: &Settings) -> Result<()> {
    let path = &settings.db_file;
    if !path.exists() {
        println!("数据库    {} (尚未创建)", path.display());
        return Ok(());
    }

    let size = std::fs::metadata(path)
        .map(|metadata| format_size(metadata.len()))
        .unwrap_or_else(|_| "大小未知".to_string());
    match Store::open(path).and_then(|store| store.count()) {
        Ok(count) => println!("数据库    {} ({count} 条记录, {size})", path.display()),
        Err(error) => println!("数据库    {} (无法读取: {error})", path.display()),
    }
    Ok(())
}

fn describe_identity(kind: Kind, partner: Option<&str>) -> String {
    match partner {
        Some(partner) => format!("{} / {partner}", kind.label()),
        None => kind.label().to_string(),
    }
}

fn describe_details(acts: &[String], protection: bool, orgasms: u32) -> String {
    let mut parts = Vec::new();
    if !acts.is_empty() {
        parts.push(acts.join(" · "));
    }
    parts.push(format!("措施 {}", if protection { "是" } else { "否" }));
    parts.push(format!("高潮 {orgasms}"));
    parts.join(" · ")
}
