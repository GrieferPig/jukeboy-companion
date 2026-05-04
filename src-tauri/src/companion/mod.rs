#[cfg(target_os = "android")]
pub(crate) mod android_ble;
pub mod client;
pub mod error;
pub mod protocol;
pub mod service;
pub mod storage;

pub use service::*;
