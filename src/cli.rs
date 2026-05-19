// CLI 命令定义模块
// 使用 clap derive 宏实现命令行解析

use clap::{Parser, Subcommand};

/// TELL-ME 命令行工具主结构体
#[derive(Parser)]
#[command(name = "tell-me", about = "tell-me - 告诉我：本地离线文档问答工具")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// 开启详细日志输出
    #[arg(long, global = true)]
    pub verbose: bool,
}

/// 子命令枚举
#[derive(Subcommand)]
pub enum Commands {
    /// 初始化文档知识库
    Init {
        /// 文档文件夹路径
        doc_path: String,
    },
    /// 提问查询
    Ask {
        /// 问题内容
        question: String,
    },
    /// 补充标准问答对
    AddFqa {
        /// 问题
        question: String,
        /// 标准答案
        answer: String,
    },
    /// 一键清空所有数据
    Clear,
}
