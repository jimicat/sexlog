use chrono::{NaiveDate, NaiveTime};
use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;

use crate::model::{self, Kind};

const EXAMPLES: &str = "\
示例:
  sexlog log
  sexlog add --date 2026-11-05 --time 22:30 --kind partner --partner A \\
    --acts \"前戏,插入\" --protection yes --orgasms 1 --satisfaction 5 --duration 40
  sexlog list --limit 10
  sexlog report
  sexlog serve --port 8787";

#[derive(Debug, Parser)]
#[command(
    name = "sexlog",
    version,
    about = "本地私密的性生活记录与数据面板",
    after_help = EXAMPLES,
    arg_required_else_help = true,
    subcommand_required = true
)]
pub struct Cli {
    /// 覆盖数据库路径（默认 ~/.sexlog/sexlog.db）
    #[arg(long, global = true, env = "SEXLOG_DB", value_name = "PATH")]
    pub db: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 记录一条数据（单条命令模式）
    Add(AddArgs),
    /// 交互式录入一条数据
    Log,
    /// 查看最近记录
    List(ListArgs),
    /// 生成静态仪表盘
    Report,
    /// 启动本地 Web 服务器查看仪表盘
    Serve(ServeArgs),
    /// 显示配置与存储路径
    Config(ConfigArgs),
}

#[derive(Debug, Args)]
pub struct AddArgs {
    /// 日期，默认今天
    #[arg(long, value_parser = model::parse_date, value_name = "YYYY-MM-DD")]
    pub date: Option<NaiveDate>,

    /// 时间，默认当前时间
    #[arg(long, value_parser = model::parse_time, value_name = "HH:MM")]
    pub time: Option<NaiveTime>,

    /// 类型：partner / solo / other
    #[arg(long, value_enum, default_value = "partner")]
    pub kind: Kind,

    /// 伴侣名称
    #[arg(long, value_name = "NAME")]
    pub partner: Option<String>,

    /// 行为标签，逗号分隔，可重复传入
    #[arg(long, value_delimiter = ',', value_name = "TAGS")]
    pub acts: Vec<String>,

    /// 是否采取保护措施：yes / no
    #[arg(
        long,
        action = clap::ArgAction::Set,
        value_parser = model::parse_yes_no,
        default_value = "no",
        value_name = "yes|no"
    )]
    pub protection: bool,

    /// 高潮次数
    #[arg(long, value_parser = model::parse_orgasms, default_value_t = 1)]
    pub orgasms: u32,

    /// 满意度（1-5）
    #[arg(long, value_parser = model::parse_satisfaction, value_name = "1-5")]
    pub satisfaction: u8,

    /// 时长（分钟）
    #[arg(long, value_parser = model::parse_duration, value_name = "MINUTES")]
    pub duration: u32,

    /// 备注
    #[arg(long, value_name = "TEXT")]
    pub notes: Option<String>,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    /// 显示条数
    #[arg(
        long,
        default_value_t = 15,
        value_parser = clap::value_parser!(u32).range(1..=500)
    )]
    pub limit: u32,
}

#[derive(Debug, Args)]
pub struct ServeArgs {
    /// 监听地址，默认 127.0.0.1
    #[arg(long, value_name = "HOST")]
    pub host: Option<String>,

    /// 监听端口，默认 8787
    #[arg(long, value_parser = model::parse_port, value_name = "PORT")]
    pub port: Option<u16>,

    /// 启动后自动打开浏览器
    #[arg(long)]
    pub open: bool,
}

#[derive(Debug, Args)]
pub struct ConfigArgs {
    /// 写入默认配置文件（不会覆盖已有文件）
    #[arg(long)]
    pub init: bool,
}
