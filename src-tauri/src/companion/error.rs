use std::io;

use thiserror::Error;

fn format_btleplug_error(error: &btleplug::Error) -> String {
    let message = error.to_string();

    #[cfg(target_os = "android")]
    {
        if matches!(error, btleplug::Error::PermissionDenied) {
            return "Android Bluetooth permission is missing or still pending. Allow Nearby devices/Bluetooth for this app and try again.".into();
        }

        let mut source: &(dyn std::error::Error + 'static) = error;
        while let Some(next) = source.source() {
            if next
                .to_string()
                .contains("Current thread is not attached to the Java VM")
            {
                return "Android BLE failed because the current Rust thread is not attached to the Android JVM.".into();
            }
            source = next;
        }
    }

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
    #[error("Android btleplug initialization failed: {0}")]
    AndroidBtleplugInit(String),
}

impl From<btleplug::Error> for CompanionError {
    fn from(error: btleplug::Error) -> Self {
        Self::Btleplug(format_btleplug_error(&error))
    }
}

pub type Result<T> = std::result::Result<T, CompanionError>;
