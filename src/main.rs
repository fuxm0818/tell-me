// tell-me - 告诉我：本地离线文档问答工具
// 主入口文件

mod cli;
mod config;
mod embedding;
mod error;
mod fqa_store;
mod handlers;
mod parser;
mod scanner;
mod splitter;
mod vector_store;

use std::env;
use std::path::PathBuf;

use clap::Parser;
use cli::{Cli, Commands};

/// 获取 tell_me_data 目录路径（程序可执行文件同级目录下）
fn get_data_dir() -> PathBuf {
    let exe_dir = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    exe_dir.join("tell_me_data")
}

fn main() {
    let cli = Cli::parse();

    if cli.verbose {
        println!("[调试] 详细日志模式已开启");
    }

    let data_dir = get_data_dir();

    match cli.command {
        Commands::Init { doc_path } => {
            if cli.verbose {
                println!("[调试] 执行 init 命令，文档路径: {}", doc_path);
                println!("[调试] 数据目录: {}", data_dir.display());
            }
            if let Err(e) = handlers::init::handle_init(&doc_path, &data_dir) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        Commands::Ask { question } => {
            if cli.verbose {
                println!("[调试] 执行 ask 命令，问题: {}", question);
            }
            if let Err(e) = handlers::ask::handle_ask(&question, &data_dir) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        Commands::AddFqa { question, answer } => {
            if cli.verbose {
                println!("[调试] 执行 add-fqa 命令，问题: {}, 答案: {}", question, answer);
            }
            if let Err(e) = handlers::add_fqa::handle_add_fqa(&question, &answer, &data_dir) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        Commands::Clear => {
            if cli.verbose {
                println!("[调试] 执行 clear 命令");
                println!("[调试] 数据目录: {}", data_dir.display());
            }
            if let Err(e) = handlers::clear::handle_clear(&data_dir) {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
    }
}
