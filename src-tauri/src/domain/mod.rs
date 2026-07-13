//! QuotaGlance 的稳定领域模型。
//!
//! 本模块不依赖 Tauri、App Server 传输或具体 UI，所有外部数据都应在
//! Provider 层完成校验后再进入这些类型。

mod error;
mod models;

pub use error::{ErrorCode, QuotaError};
pub use models::{
    AuthSummary, AuthUiState, CreditSummary, ProviderKind, QuotaBucket, QuotaSnapshot, QuotaSource,
    QuotaStatus, QuotaWindow, ResetCreditDetail, ResetCreditSummary, WindowKind, WindowSlot,
};
