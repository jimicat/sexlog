mod cli;
mod commands;
mod config;
mod db;
mod interactive;
mod model;
mod report;
mod server;
mod util;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Command};

fn main() {
    #[cfg(unix)]
    // 恢复默认 SIGPIPE 行为，避免输出被 head 等命令截断时 panic。
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }

    if let Err(error) = run() {
        eprintln!("错误: {error:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let settings = config::Settings::load(cli.db)?;

    match cli.command {
        Command::Add(args) => commands::add(args, &settings),
        Command::Log => commands::log(&settings),
        Command::List(args) => commands::list(args, &settings),
        Command::Report => commands::report(&settings),
        Command::Serve(args) => commands::serve(args, &settings),
        Command::Config(args) => commands::config(args, &settings),
    }
}
