use std::io;

use thiserror::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TransportFailureKind {
    AdapterUnavailable,
    NotConnected,
}

fn classify_transport_message(message: &str) -> Option<TransportFailureKind> {
    let normalized = message.to_ascii_lowercase();

    if normalized.contains("object has been closed")
        || normalized.contains("notification stream ended")
        || normalized.contains("ble device is not connected")
        || normalized.contains("ble device disconnected")
        || normalized.contains("device is not ready for use")
        || normalized.contains("ble session is not connected")
        || normalized.contains("peripheral disconnected")
        || (normalized.contains("ble session") && normalized.contains("not available"))
    {
        return Some(TransportFailureKind::NotConnected);
    }

    if normalized.contains("no bluetooth adapter")
        || normalized.contains("bluetooth adapter is unavailable")
        || normalized.contains("bluetooth adapter is disabled")
        || normalized.contains("bluetooth is disabled")
    {
        return Some(TransportFailureKind::AdapterUnavailable);
    }

    None
}

#[cfg(not(target_os = "android"))]
fn format_btleplug_error(error: &btleplug::Error) -> String {
    let message = error.to_string();

    format!("BTLE plug error: {message}")
}

#[derive(Debug, Error)]
pub enum CompanionError {
    #[error("no Bluetooth adapter is available")]
    NoBluetoothAdapter,
    #[error("no Jukeboy companion BLE device found")]
    DeviceNotFound,
    #[error("BLE device is not connected")]
    NotConnected,
    #[error("BLE protocol error: {0}")]
    Protocol(String),
    #[error("{0}")]
    Btleplug(String),
    #[error("Android BLE bridge error: {0}")]
    AndroidBleBridge(String),
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("operation timed out")]
    Timeout,
    #[error("authentication secret must be 32 bytes in hex")]
    InvalidSecretHex,
    #[error("requested profile has no credentials")]
    MissingCredentials,
    #[error("pairing button sequence must contain exactly 4 items")]
    InvalidButtonSequence,
    #[error("unknown button name: {0}")]
    UnknownButton(String),
    #[error("unknown playback mode: {0}")]
    UnknownPlaybackMode(String),
    #[error("unknown output target: {0}")]
    UnknownOutputTarget(String),
    #[error("unknown playback action: {0}")]
    UnknownPlaybackAction(String),
    #[error("unknown Last.fm action: {0}")]
    UnknownLastfmAction(String),
    #[error("unknown Bluetooth action: {0}")]
    UnknownBluetoothAction(String),
    #[error("authentication challenge nonce was invalid")]
    InvalidNonce,
    #[error("companion API error opcode=0x{opcode:04x} request_id={request_id}: {message} ({error_code})")]
    Api {
        opcode: u16,
        request_id: u32,
        error_code: i32,
        message: String,
    },
    #[error("application data path is unavailable")]
    AppDataPathUnavailable,
}

impl CompanionError {
    pub fn android_ble_bridge(message: impl Into<String>) -> Self {
        let message = message.into();

        match classify_transport_message(&message) {
            Some(TransportFailureKind::NotConnected) => Self::NotConnected,
            Some(TransportFailureKind::AdapterUnavailable) => Self::NoBluetoothAdapter,
            None => Self::AndroidBleBridge(message),
        }
    }
}

#[cfg(not(target_os = "android"))]
impl From<btleplug::Error> for CompanionError {
    fn from(error: btleplug::Error) -> Self {
        match classify_transport_message(&error.to_string()) {
            Some(TransportFailureKind::NotConnected) => Self::NotConnected,
            Some(TransportFailureKind::AdapterUnavailable) => Self::NoBluetoothAdapter,
            None => Self::Btleplug(format_btleplug_error(&error)),
        }
    }
}

pub type Result<T> = std::result::Result<T, CompanionError>;

#[cfg(test)]
mod tests {
    use super::{classify_transport_message, TransportFailureKind};

    #[test]
    fn classifies_closed_transport_as_not_connected() {
        let message =
            "BTLE plug error: Error { code: HRESULT(0x80000013), message: \"The object has been closed.\" }";

        assert_eq!(
            classify_transport_message(message),
            Some(TransportFailureKind::NotConnected)
        );
    }

    #[test]
    fn classifies_unready_peripheral_as_not_connected() {
        let message =
            "BTLE plug error: Error { code: HRESULT(0x800710DF), message: \"The device is not ready for use.\" }";

        assert_eq!(
            classify_transport_message(message),
            Some(TransportFailureKind::NotConnected)
        );
    }

    #[test]
    fn classifies_android_session_closure_as_not_connected() {
        assert_eq!(
            classify_transport_message("Android BLE write failed: BLE session 7 is not available"),
            Some(TransportFailureKind::NotConnected)
        );
    }

    #[test]
    fn classifies_disabled_adapter_as_unavailable() {
        assert_eq!(
            classify_transport_message("Android BLE connect failed: Bluetooth adapter is disabled"),
            Some(TransportFailureKind::AdapterUnavailable)
        );
    }
}
