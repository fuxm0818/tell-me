// 统一错误类型定义模块
// 使用 thiserror 定义 CoiError 枚举，包含所有错误变体

use thiserror::Error;

/// COI 统一错误类型
/// 所有错误信息使用中文格式化，包含原因和建议
#[derive(Error, Debug)]
pub enum CoiError {
    /// 路径无效错误：指定的文件夹不存在
    #[error("[错误] 路径无效: {path}\n  原因: 指定的文件夹不存在\n  建议: 请检查路径是否正确")]
    InvalidPath { path: String },

    /// 未初始化错误：未找到配置文件
    #[error("[错误] 未初始化\n  原因: 未找到配置文件\n  建议: 请先执行 coi init <文档文件夹路径>")]
    NotInitialized,

    /// 输入无效错误：用户输入不符合要求
    #[error("[错误] 输入无效\n  原因: {reason}\n  建议: 请输入有效的内容")]
    InvalidInput { reason: String },

    /// 文件解析失败错误：文档格式异常或损坏
    #[error("[错误] 文件解析失败: {file}\n  原因: {reason}")]
    ParseError { file: String, reason: String },

    /// 模型加载失败错误：模型文件缺失或损坏
    #[error("[错误] 模型加载失败\n  原因: {reason}\n  建议: 请确认 model/ 目录下模型文件完整")]
    ModelError { reason: String },

    /// 删除失败错误：文件被占用或权限不足
    #[error("[错误] 删除失败\n  原因: {reason}\n  建议: 请检查文件权限或关闭占用程序")]
    ClearError { reason: String },

    /// 其他错误：从 anyhow::Error 自动转换
    #[error("[错误] {0}")]
    Other(#[from] anyhow::Error),
}
