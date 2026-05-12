use std::time::Duration;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use hmac::{Hmac, Mac};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::Sha256;
use uuid::Uuid;

use crate::error::{CompanionError, Result};

type HmacSha256 = Hmac<Sha256>;

pub const DEFAULT_DEVICE_NAME: &str = "ESP_SPP_SERVER";
pub const DEFAULT_TIMEOUT_SECS: f64 = 10.0;
pub const DEFAULT_SCAN_TIMEOUT_SECS: f64 = 5.0;
pub const DEFAULT_CHUNK_SIZE: usize = 20;
pub const FRAME_MAX_LEN: usize = 2048;
pub const PAIR_SECRET_LEN: usize = 32;
pub const AUTH_NONCE_LEN: usize = 16;
pub const MAGIC: [u8; 2] = *b"JC";
pub const VERSION: u8 = 1;
pub const FRAME_HEADER_LEN: usize = 12;
pub const TLV_HEADER_LEN: usize = 4;
pub const EVENT_NAME: &str = "companion://frame";

pub fn service_uuid() -> Uuid {
    Uuid::from_u128(0x0000abf000001000800000805f9b34fb)
}

pub fn write_uuid() -> Uuid {
    Uuid::from_u128(0x0000abf100001000800000805f9b34fb)
}

pub fn notify_uuid() -> Uuid {
    Uuid::from_u128(0x0000abf200001000800000805f9b34fb)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum FrameType {
    Request = 1,
    Response = 2,
    Event = 3,
    Heartbeat = 4,
    Error = 5,
}

impl TryFrom<u8> for FrameType {
    type Error = CompanionError;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            1 => Ok(Self::Request),
            2 => Ok(Self::Response),
            3 => Ok(Self::Event),
            4 => Ok(Self::Heartbeat),
            5 => Ok(Self::Error),
            _ => Err(CompanionError::Protocol(format!(
                "unknown frame type {value}"
            ))),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum Opcode {
    Hello = 0x0001,
    Capabilities = 0x0002,
    Ping = 0x0003,
    PairBegin = 0x0010,
    PairStatus = 0x0011,
    PairCancel = 0x0012,
    AuthChallenge = 0x0013,
    AuthProof = 0x0014,
    TrustedList = 0x0015,
    TrustedRevoke = 0x0016,
    Snapshot = 0x0100,
    PlaybackStatus = 0x0101,
    PlaybackControl = 0x0102,
    LibraryAlbum = 0x0110,
    LibraryTrackPage = 0x0111,
    WifiStatus = 0x0200,
    WifiScanStart = 0x0201,
    WifiScanResults = 0x0202,
    WifiConnect = 0x0203,
    WifiConnectSlot = 0x0204,
    WifiDisconnect = 0x0205,
    WifiAutoreconnect = 0x0206,
    LastfmStatus = 0x0300,
    LastfmControl = 0x0301,
    HistorySummary = 0x0400,
    HistoryAlbumPage = 0x0401,
    BtAudioStatus = 0x0500,
    BtAudioControl = 0x0501,
    OutputStatus = 0x0103,
    OutputSelect = 0x0104,
    WifiListSlots = 0x0207,
    WifiSaveSlot = 0x0208,
    WifiReconnect = 0x0209,
    LastfmRequestToken = 0x0302,
    HistoryTrackPage = 0x0402,
    HistoryClear = 0x0403,
    BtScanStart = 0x0502,
    BtScanResults = 0x0503,
    BtBondedList = 0x0504,
    BtUnbond = 0x0505,
    HidStatus = 0x0600,
    HidLedSet = 0x0601,
    ScriptStatus = 0x0700,
    ScriptList = 0x0701,
    ScriptLog = 0x0702,
    ScriptRun = 0x0703,
    SystemReboot = 0x0F00,
    SystemRebootDownload = 0x0F01,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u16)]
pub enum TlvType {
    ErrorCode = 0x0002,
    ErrorMessage = 0x0003,
    ProtocolVersion = 0x0004,
    FeatureBits = 0x0005,
    MaxFrame = 0x0006,
    Mtu = 0x0007,
    MaxPayload = 0x0008,
    Authenticated = 0x0100,
    ClientId = 0x0101,
    AppName = 0x0102,
    SharedSecret = 0x0103,
    ButtonSequence = 0x0104,
    PairingPending = 0x0105,
    PairingProgress = 0x0106,
    PairingRequired = 0x0107,
    AuthNonce = 0x0108,
    AuthHmac = 0x0109,
    TrustedCount = 0x010A,
    CreatedAt = 0x010B,
    Generation = 0x0200,
    UptimeMs = 0x0201,
    QueueFree = 0x0202,
    RxFrames = 0x0203,
    TxFrames = 0x0204,
    RxErrors = 0x0205,
    Playing = 0x0300,
    Paused = 0x0301,
    TrackIndex = 0x0302,
    TrackCount = 0x0303,
    PositionSec = 0x0304,
    StartedAt = 0x0305,
    DurationSec = 0x0306,
    VolumePercent = 0x0307,
    PlaybackMode = 0x0308,
    TrackTitle = 0x0309,
    TrackArtist = 0x030A,
    TrackFile = 0x030B,
    Action = 0x030C,
    Value = 0x030D,
    OutputTarget = 0x030E,
    CartridgeStatus = 0x0400,
    CartridgeMounted = 0x0401,
    CartridgeChecksum = 0x0402,
    MetadataVersion = 0x0403,
    AlbumName = 0x0404,
    AlbumArtist = 0x0405,
    AlbumDescription = 0x0406,
    AlbumYear = 0x0407,
    AlbumDuration = 0x0408,
    AlbumGenre = 0x0409,
    Offset = 0x040A,
    Count = 0x040B,
    ReturnedCount = 0x040C,
    WifiState = 0x0500,
    WifiInternet = 0x0501,
    WifiAutoreconnect = 0x0502,
    WifiActiveSlot = 0x0503,
    WifiPreferredSlot = 0x0504,
    WifiIp = 0x0505,
    WifiSsid = 0x0506,
    WifiPassword = 0x0507,
    WifiSlot = 0x0508,
    WifiRssi = 0x0509,
    WifiChannel = 0x050A,
    WifiAuthMode = 0x050B,
    LastfmHasAuthUrl = 0x0600,
    LastfmHasToken = 0x0601,
    LastfmHasSession = 0x0602,
    LastfmBusy = 0x0603,
    LastfmScrobbling = 0x0604,
    LastfmNowPlaying = 0x0605,
    LastfmPendingCommands = 0x0606,
    LastfmPendingScrobbles = 0x0607,
    LastfmSuccessful = 0x0608,
    LastfmFailed = 0x0609,
    LastfmAuthUrl = 0x060A,
    LastfmUsername = 0x060B,
    HistoryAlbumCount = 0x0700,
    HistoryTrackCount = 0x0701,
    HistoryPlayCount = 0x0702,
    HistoryFirstSeen = 0x0703,
    HistoryLastSeen = 0x0704,
    BtA2dpConnected = 0x0800,
    BtBondedCount = 0x0801,
    BtAddr = 0x0802,
    BtName = 0x0803,
    BtRssi = 0x0804,
    BtCod = 0x0805,
    BtScanRunning = 0x0806,
    WifiHasPassword = 0x0900,
    WifiSlotConfigured = 0x0901,
    WifiSlotPreferred = 0x0902,
    WifiSlotActive = 0x0903,
    HidButtonBitmap = 0x0A00,
    HidAdcRaw = 0x0A01,
    HidLedR = 0x0A02,
    HidLedG = 0x0A03,
    HidLedB = 0x0A04,
    HidLedBrightness = 0x0A05,
    HidLedOff = 0x0A06,
    ScriptState = 0x0B00,
    ScriptName = 0x0B01,
    ScriptResolvedPath = 0x0B02,
    ScriptMessage = 0x0B03,
    ScriptOutput = 0x0B04,
    ScriptRunId = 0x0B05,
    ScriptExitCode = 0x0B06,
    ScriptKind = 0x0B07,
    ScriptSize = 0x0B08,
    ScriptArgs = 0x0B09,
    OutputTargetResult = 0x0C00,
    TotalSize = 0x040D,
    BinaryData = 0x040E,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum PlaybackAction {
    Next = 1,
    Previous = 2,
    PauseToggle = 3,
    FastForward = 4,
    Rewind = 5,
    PlayIndex = 6,
    SeekSeconds = 7,
    SetVolumePercent = 8,
    SetMode = 9,
    SetOutputTarget = 10,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum LastfmAction {
    SetAuthUrl = 1,
    RequestToken = 2,
    Auth = 3,
    Logout = 4,
    SetScrobbling = 5,
    SetNowPlaying = 6,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum BtAction {
    ConnectLast = 1,
    PairBest = 2,
    Disconnect = 3,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tlv {
    pub tlv_type: u16,
    pub value: Vec<u8>,
}

impl Tlv {
    pub fn name(&self) -> String {
        tlv_name(self.tlv_type)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Frame {
    pub frame_type: u8,
    pub opcode: u16,
    pub request_id: u32,
    pub payload: Vec<u8>,
    pub tlvs: Option<Vec<Tlv>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompanionCredentials {
    pub client_id: String,
    pub app_name: String,
    pub secret_hex: String,
}

impl CompanionCredentials {
    pub fn secret(&self) -> Result<Vec<u8>> {
        let bytes = hex::decode(&self.secret_hex).map_err(|_| CompanionError::InvalidSecretHex)?;
        if bytes.len() != PAIR_SECRET_LEN {
            return Err(CompanionError::InvalidSecretHex);
        }
        Ok(bytes)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    pub address: String,
    pub name: String,
    pub service_match: bool,
    pub uuids: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConnectedDevice {
    pub address: String,
    pub name: String,
    pub profile: String,
}

pub fn default_timeout() -> Duration {
    Duration::from_secs_f64(DEFAULT_TIMEOUT_SECS)
}

pub fn default_scan_timeout() -> Duration {
    Duration::from_secs_f64(DEFAULT_SCAN_TIMEOUT_SECS)
}

pub fn write_u16(value: u16) -> [u8; 2] {
    value.to_le_bytes()
}

pub fn write_u32(value: u32) -> [u8; 4] {
    value.to_le_bytes()
}

pub fn read_u16(data: &[u8]) -> Result<u16> {
    if data.len() < 2 {
        return Err(CompanionError::Protocol("expected u16".into()));
    }
    Ok(u16::from_le_bytes([data[0], data[1]]))
}

pub fn read_u32(data: &[u8]) -> Result<u32> {
    if data.len() < 4 {
        return Err(CompanionError::Protocol("expected u32".into()));
    }
    Ok(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
}

pub fn read_i32_from_u32(value: u32) -> i32 {
    i32::from_le_bytes(value.to_le_bytes())
}

pub fn tlv_bytes(tlv_type: u16, value: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(TLV_HEADER_LEN + value.len());
    encoded.extend_from_slice(&write_u16(tlv_type));
    encoded.extend_from_slice(&write_u16(value.len() as u16));
    encoded.extend_from_slice(value);
    encoded
}

pub fn tlv_u8(tlv_type: u16, value: u8) -> Vec<u8> {
    tlv_bytes(tlv_type, &[value])
}

pub fn tlv_u16(tlv_type: u16, value: u16) -> Vec<u8> {
    tlv_bytes(tlv_type, &write_u16(value))
}

pub fn tlv_u32(tlv_type: u16, value: u32) -> Vec<u8> {
    tlv_bytes(tlv_type, &write_u32(value))
}

pub fn tlv_string(tlv_type: u16, value: &str) -> Vec<u8> {
    tlv_bytes(tlv_type, value.as_bytes())
}

pub fn parse_tlvs(payload: &[u8]) -> Result<Vec<Tlv>> {
    let mut tlvs = Vec::new();
    let mut offset = 0usize;
    while offset + TLV_HEADER_LEN <= payload.len() {
        let tlv_type = read_u16(&payload[offset..offset + 2])?;
        let tlv_len = read_u16(&payload[offset + 2..offset + 4])? as usize;
        offset += TLV_HEADER_LEN;
        if offset + tlv_len > payload.len() {
            return Err(CompanionError::Protocol(
                "TLV exceeds payload length".into(),
            ));
        }
        tlvs.push(Tlv {
            tlv_type,
            value: payload[offset..offset + tlv_len].to_vec(),
        });
        offset += tlv_len;
    }
    if offset != payload.len() {
        return Err(CompanionError::Protocol(
            "trailing bytes after TLV parse".into(),
        ));
    }
    Ok(tlvs)
}

pub fn decode_frame_bytes(raw: &[u8]) -> Result<Frame> {
    if raw.len() < FRAME_HEADER_LEN {
        return Err(CompanionError::Protocol("frame too short".into()));
    }
    if raw[0..2] != MAGIC {
        return Err(CompanionError::Protocol("invalid frame magic".into()));
    }
    if raw[2] != VERSION {
        return Err(CompanionError::Protocol("unsupported frame version".into()));
    }
    let frame_type = FrameType::try_from(raw[3])?;
    let opcode = read_u16(&raw[4..6])?;
    let request_id = read_u32(&raw[6..10])?;
    let payload_len = read_u16(&raw[10..12])? as usize;
    if FRAME_HEADER_LEN + payload_len > raw.len() {
        return Err(CompanionError::Protocol(
            "payload length exceeds frame size".into(),
        ));
    }
    let payload = raw[12..12 + payload_len].to_vec();
    let tlvs = if frame_type != FrameType::Response || opcode != Opcode::Ping as u16 {
        parse_tlvs(&payload).ok()
    } else {
        None
    };
    Ok(Frame {
        frame_type: frame_type as u8,
        opcode,
        request_id,
        payload,
        tlvs,
    })
}

pub fn encode_request_frame(opcode: u16, request_id: u32, payload: &[u8]) -> Vec<u8> {
    let mut encoded = Vec::with_capacity(FRAME_HEADER_LEN + payload.len());
    encoded.extend_from_slice(&MAGIC);
    encoded.push(VERSION);
    encoded.push(FrameType::Request as u8);
    encoded.extend_from_slice(&write_u16(opcode));
    encoded.extend_from_slice(&write_u32(request_id));
    encoded.extend_from_slice(&write_u16(payload.len() as u16));
    encoded.extend_from_slice(payload);
    encoded
}

pub fn tlv_first<'a>(tlvs: &'a [Tlv], tlv_type: u16) -> Option<&'a Tlv> {
    tlvs.iter().find(|tlv| tlv.tlv_type == tlv_type)
}

pub fn tlv_value_u8(tlv: &Tlv) -> Result<u8> {
    if tlv.value.len() != 1 {
        return Err(CompanionError::Protocol(format!(
            "TLV {} is not u8",
            tlv.name()
        )));
    }
    Ok(tlv.value[0])
}

pub fn tlv_value_u16(tlv: &Tlv) -> Result<u16> {
    if tlv.value.len() != 2 {
        return Err(CompanionError::Protocol(format!(
            "TLV {} is not u16",
            tlv.name()
        )));
    }
    read_u16(&tlv.value)
}

pub fn tlv_value_u32(tlv: &Tlv) -> Result<u32> {
    if tlv.value.len() != 4 {
        return Err(CompanionError::Protocol(format!(
            "TLV {} is not u32",
            tlv.name()
        )));
    }
    read_u32(&tlv.value)
}

pub fn tlv_value_bool(tlv: &Tlv) -> Result<bool> {
    Ok(tlv_value_u8(tlv)? != 0)
}

pub fn tlv_value_string(tlv: &Tlv) -> String {
    String::from_utf8_lossy(&tlv.value).into_owned()
}

pub fn decode_slot(value: u8) -> Option<u8> {
    if value == 0 {
        None
    } else {
        Some(value - 1)
    }
}

pub fn decode_ip_address(value: u32) -> String {
    let octets = value.to_le_bytes();
    format!("{}.{}.{}.{}", octets[0], octets[1], octets[2], octets[3])
}

pub fn ensure_track_index(value: u32) -> Option<u32> {
    if value == u32::MAX {
        None
    } else {
        Some(value)
    }
}

pub fn button_sequence_names(sequence: &[u8]) -> Vec<String> {
    sequence
        .iter()
        .map(|value| button_id_to_name(*value).to_string())
        .collect()
}

pub fn button_id_to_name(value: u8) -> &'static str {
    match value {
        0 => "main1",
        1 => "main2",
        2 => "main3",
        3 => "misc1",
        4 => "misc2",
        5 => "misc3",
        _ => "unknown",
    }
}

pub fn button_name_to_id(value: &str) -> Option<u8> {
    match value {
        "main1" => Some(0),
        "main2" => Some(1),
        "main3" => Some(2),
        "misc1" => Some(3),
        "misc2" => Some(4),
        "misc3" => Some(5),
        _ => None,
    }
}

pub fn parse_button_sequence(sequence: Option<&[String]>) -> Result<Vec<u8>> {
    match sequence {
        Some(values) => {
            if values.len() != 4 {
                return Err(CompanionError::InvalidButtonSequence);
            }
            values
                .iter()
                .map(|value| {
                    button_name_to_id(&value.to_lowercase())
                        .ok_or_else(|| CompanionError::UnknownButton(value.clone()))
                })
                .collect()
        }
        None => {
            let mut rng = rand::thread_rng();
            Ok((0..4).map(|_| rng.gen_range(0..6)).collect())
        }
    }
}

pub fn playback_mode_to_id(value: &str) -> Result<u8> {
    match value {
        "sequential" => Ok(0),
        "single_repeat" => Ok(1),
        "shuffle" => Ok(2),
        _ => Err(CompanionError::UnknownPlaybackMode(value.to_string())),
    }
}

pub fn playback_mode_from_id(value: u8) -> Value {
    match value {
        0 => json!("sequential"),
        1 => json!("single_repeat"),
        2 => json!("shuffle"),
        _ => json!(value),
    }
}

pub fn output_target_to_id(value: &str) -> Result<u8> {
    match value {
        "bluetooth" => Ok(0),
        "i2s" => Ok(1),
        _ => Err(CompanionError::UnknownOutputTarget(value.to_string())),
    }
}

pub fn output_target_from_id(value: u8) -> Value {
    match value {
        0 => json!("bluetooth"),
        1 => json!("i2s"),
        _ => json!(value),
    }
}

pub fn wifi_state_from_id(value: u8) -> Value {
    match value {
        0 => json!("idle"),
        1 => json!("scanning"),
        2 => json!("connecting"),
        3 => json!("connected"),
        4 => json!("disconnected"),
        _ => json!(value),
    }
}

pub fn cartridge_status_from_id(value: u8) -> Value {
    match value {
        0 => json!("empty"),
        1 => json!("ready"),
        2 => json!("invalid"),
        _ => json!(value),
    }
}

pub fn frame_type_name(frame_type: u8) -> String {
    match FrameType::try_from(frame_type) {
        Ok(FrameType::Request) => "request".into(),
        Ok(FrameType::Response) => "response".into(),
        Ok(FrameType::Event) => "event".into(),
        Ok(FrameType::Heartbeat) => "heartbeat".into(),
        Ok(FrameType::Error) => "error".into(),
        Err(_) => format!("0x{frame_type:02x}"),
    }
}

pub fn opcode_name(opcode: u16) -> String {
    match opcode {
        value if value == Opcode::Hello as u16 => "hello",
        value if value == Opcode::Capabilities as u16 => "capabilities",
        value if value == Opcode::Ping as u16 => "ping",
        value if value == Opcode::PairBegin as u16 => "pair_begin",
        value if value == Opcode::PairStatus as u16 => "pair_status",
        value if value == Opcode::PairCancel as u16 => "pair_cancel",
        value if value == Opcode::AuthChallenge as u16 => "auth_challenge",
        value if value == Opcode::AuthProof as u16 => "auth_proof",
        value if value == Opcode::TrustedList as u16 => "trusted_list",
        value if value == Opcode::TrustedRevoke as u16 => "trusted_revoke",
        value if value == Opcode::Snapshot as u16 => "snapshot",
        value if value == Opcode::PlaybackStatus as u16 => "playback_status",
        value if value == Opcode::PlaybackControl as u16 => "playback_control",
        value if value == Opcode::LibraryAlbum as u16 => "library_album",
        value if value == Opcode::LibraryTrackPage as u16 => "library_track_page",
        value if value == Opcode::WifiStatus as u16 => "wifi_status",
        value if value == Opcode::WifiScanStart as u16 => "wifi_scan_start",
        value if value == Opcode::WifiScanResults as u16 => "wifi_scan_results",
        value if value == Opcode::WifiConnect as u16 => "wifi_connect",
        value if value == Opcode::WifiConnectSlot as u16 => "wifi_connect_slot",
        value if value == Opcode::WifiDisconnect as u16 => "wifi_disconnect",
        value if value == Opcode::WifiAutoreconnect as u16 => "wifi_autoreconnect",
        value if value == Opcode::LastfmStatus as u16 => "lastfm_status",
        value if value == Opcode::LastfmControl as u16 => "lastfm_control",
        value if value == Opcode::HistorySummary as u16 => "history_summary",
        value if value == Opcode::HistoryAlbumPage as u16 => "history_album_page",
        value if value == Opcode::BtAudioStatus as u16 => "bt_audio_status",
        value if value == Opcode::BtAudioControl as u16 => "bt_audio_control",
        value if value == Opcode::OutputStatus as u16 => "output_status",
        value if value == Opcode::OutputSelect as u16 => "output_select",
        value if value == Opcode::WifiListSlots as u16 => "wifi_list_slots",
        value if value == Opcode::WifiSaveSlot as u16 => "wifi_save_slot",
        value if value == Opcode::WifiReconnect as u16 => "wifi_reconnect",
        value if value == Opcode::LastfmRequestToken as u16 => "lastfm_request_token",
        value if value == Opcode::HistoryTrackPage as u16 => "history_track_page",
        value if value == Opcode::HistoryClear as u16 => "history_clear",
        value if value == Opcode::BtScanStart as u16 => "bt_scan_start",
        value if value == Opcode::BtScanResults as u16 => "bt_scan_results",
        value if value == Opcode::BtBondedList as u16 => "bt_bonded_list",
        value if value == Opcode::BtUnbond as u16 => "bt_unbond",
        value if value == Opcode::HidStatus as u16 => "hid_status",
        value if value == Opcode::HidLedSet as u16 => "hid_led_set",
        value if value == Opcode::ScriptStatus as u16 => "script_status",
        value if value == Opcode::ScriptList as u16 => "script_list",
        value if value == Opcode::ScriptLog as u16 => "script_log",
        value if value == Opcode::ScriptRun as u16 => "script_run",
        value if value == Opcode::SystemReboot as u16 => "system_reboot",
        value if value == Opcode::SystemRebootDownload as u16 => "system_reboot_download",
        _ => return format!("0x{opcode:04x}"),
    }
    .to_string()
}

pub fn tlv_name(tlv_type: u16) -> String {
    match tlv_type {
        0x0001 => "status",
        0x0002 => "error_code",
        0x0003 => "error_message",
        0x0004 => "protocol_version",
        0x0005 => "feature_bits",
        0x0006 => "max_frame",
        0x0007 => "mtu",
        0x0008 => "max_payload",
        0x0009 => "request_id",
        0x0100 => "authenticated",
        0x0101 => "client_id",
        0x0102 => "app_name",
        0x0103 => "shared_secret",
        0x0104 => "button_sequence",
        0x0105 => "pairing_pending",
        0x0106 => "pairing_progress",
        0x0107 => "pairing_required",
        0x0108 => "auth_nonce",
        0x0109 => "auth_hmac",
        0x010A => "trusted_count",
        0x010B => "created_at",
        0x0200 => "generation",
        0x0201 => "uptime_ms",
        0x0202 => "queue_free",
        0x0203 => "rx_frames",
        0x0204 => "tx_frames",
        0x0205 => "rx_errors",
        0x0300 => "playing",
        0x0301 => "paused",
        0x0302 => "track_index",
        0x0303 => "track_count",
        0x0304 => "position_sec",
        0x0305 => "started_at",
        0x0306 => "duration_sec",
        0x0307 => "volume_percent",
        0x0308 => "playback_mode",
        0x0309 => "track_title",
        0x030A => "track_artist",
        0x030B => "track_file",
        0x030C => "action",
        0x030D => "value",
        0x030E => "output_target",
        0x0400 => "cartridge_status",
        0x0401 => "cartridge_mounted",
        0x0402 => "cartridge_checksum",
        0x0403 => "metadata_version",
        0x0404 => "album_name",
        0x0405 => "album_artist",
        0x0406 => "album_description",
        0x0407 => "album_year",
        0x0408 => "album_duration",
        0x0409 => "album_genre",
        0x040A => "offset",
        0x040B => "count",
        0x040C => "returned_count",
        0x0500 => "wifi_state",
        0x0501 => "wifi_internet",
        0x0502 => "wifi_autoreconnect",
        0x0503 => "wifi_active_slot",
        0x0504 => "wifi_preferred_slot",
        0x0505 => "wifi_ip",
        0x0506 => "wifi_ssid",
        0x0507 => "wifi_password",
        0x0508 => "wifi_slot",
        0x0509 => "wifi_rssi",
        0x050A => "wifi_channel",
        0x050B => "wifi_authmode",
        0x0600 => "lastfm_has_auth_url",
        0x0601 => "lastfm_has_token",
        0x0602 => "lastfm_has_session",
        0x0603 => "lastfm_busy",
        0x0604 => "lastfm_scrobbling",
        0x0605 => "lastfm_now_playing",
        0x0606 => "lastfm_pending_commands",
        0x0607 => "lastfm_pending_scrobbles",
        0x0608 => "lastfm_successful",
        0x0609 => "lastfm_failed",
        0x060A => "lastfm_auth_url",
        0x060B => "lastfm_username",
        0x0700 => "history_album_count",
        0x0701 => "history_track_count",
        0x0702 => "history_play_count",
        0x0703 => "history_first_seen",
        0x0704 => "history_last_seen",
        0x0800 => "bt_a2dp_connected",
        0x0801 => "bt_bonded_count",
        0x0802 => "bt_addr",
        0x0803 => "bt_name",
        0x0804 => "bt_rssi",
        0x0805 => "bt_cod",
        0x0806 => "bt_scan_running",
        0x0900 => "wifi_has_password",
        0x0901 => "wifi_slot_configured",
        0x0902 => "wifi_slot_preferred",
        0x0903 => "wifi_slot_active",
        0x0A00 => "hid_button_bitmap",
        0x0A01 => "hid_adc_raw",
        0x0A02 => "hid_led_r",
        0x0A03 => "hid_led_g",
        0x0A04 => "hid_led_b",
        0x0A05 => "hid_led_brightness",
        0x0A06 => "hid_led_off",
        0x0B00 => "script_state",
        0x0B01 => "script_name",
        0x0B02 => "script_resolved_path",
        0x0B03 => "script_message",
        0x0B04 => "script_output",
        0x0B05 => "script_run_id",
        0x0B06 => "script_exit_code",
        0x0B07 => "script_kind",
        0x0B08 => "script_size",
        0x0B09 => "script_args",
        0x040D => "total_size",
        0x040E => "binary_data",
        0x0C00 => "output_target_result",
        _ => return format!("0x{tlv_type:04x}"),
    }
    .to_string()
}

pub fn generate_pairing_credentials(
    client_id: Option<String>,
    app_name: Option<String>,
    secret_hex: Option<String>,
) -> CompanionCredentials {
    CompanionCredentials {
        client_id: client_id.unwrap_or_else(|| Uuid::new_v4().to_string()),
        app_name: app_name.unwrap_or_else(|| "jukeboy-companion".to_string()),
        secret_hex: secret_hex
            .unwrap_or_else(|| hex::encode(rand::thread_rng().gen::<[u8; PAIR_SECRET_LEN]>())),
    }
}

pub fn build_auth_proof(secret: &[u8], nonce: &[u8]) -> Result<Vec<u8>> {
    let mut mac = HmacSha256::new_from_slice(secret)
        .map_err(|_| CompanionError::Protocol("invalid HMAC secret".into()))?;
    mac.update(nonce);
    Ok(mac.finalize().into_bytes().to_vec())
}

pub fn decode_auth_status(frame: &Frame) -> Result<Value> {
    let mut result = json!({
        "opcode": opcode_name(frame.opcode),
        "request_id": frame.request_id,
        "authenticated": false,
        "client_id": "",
        "trusted_client_count": 0
    });
    for tlv in frame.tlvs.as_deref().unwrap_or(&[]) {
        match tlv.tlv_type {
            value if value == TlvType::Authenticated as u16 => {
                result["authenticated"] = json!(tlv_value_bool(tlv)?)
            }
            value if value == TlvType::ClientId as u16 => {
                result["client_id"] = json!(tlv_value_string(tlv))
            }
            value if value == TlvType::TrustedCount as u16 => {
                result["trusted_client_count"] = json!(tlv_value_u8(tlv)?)
            }
            _ => {}
        }
    }
    Ok(result)
}

pub fn decode_hello(frame: &Frame) -> Result<Value> {
    let mut result = decode_auth_status(frame)?;
    result["app_name"] = json!("");
    result["protocol_version"] = Value::Null;
    for tlv in frame.tlvs.as_deref().unwrap_or(&[]) {
        match tlv.tlv_type {
            value if value == TlvType::AppName as u16 => {
                result["app_name"] = json!(tlv_value_string(tlv))
            }
            value if value == TlvType::ProtocolVersion as u16 => {
                result["protocol_version"] = json!(tlv_value_u16(tlv)?)
            }
            _ => {}
        }
    }
    Ok(result)
}

pub fn decode_pair_status(frame: &Frame) -> Result<Value> {
    let mut result = json!({
        "opcode": opcode_name(frame.opcode),
        "request_id": frame.request_id,
        "pairing_pending": false,
        "pairing_progress": 0,
        "pairing_required": 0,
        "pending_client_id": "",
        "pending_app_name": "",
        "button_sequence": []
    });
    for tlv in frame.tlvs.as_deref().unwrap_or(&[]) {
        match tlv.tlv_type {
            value if value == TlvType::PairingPending as u16 => {
                result["pairing_pending"] = json!(tlv_value_bool(tlv)?)
            }
            value if value == TlvType::PairingProgress as u16 => {
                result["pairing_progress"] = json!(tlv_value_u8(tlv)?)
            }
            value if value == TlvType::PairingRequired as u16 => {
                result["pairing_required"] = json!(tlv_value_u8(tlv)?)
            }
            value if value == TlvType::ClientId as u16 => {
                result["pending_client_id"] = json!(tlv_value_string(tlv))
            }
            value if value == TlvType::AppName as u16 => {
                result["pending_app_name"] = json!(tlv_value_string(tlv))
            }
            value if value == TlvType::ButtonSequence as u16 => {
                result["button_sequence"] = json!(button_sequence_names(&tlv.value))
            }
            _ => {}
        }
    }
    Ok(result)
}

pub fn decode_capabilities(frame: &Frame) -> Result<Value> {
    let mut result = decode_auth_status(frame)?;
    result["protocol_version"] = Value::Null;
    result["max_frame"] = Value::Null;
    result["mtu"] = Value::Null;
    result["max_payload"] = Value::Null;
    result["feature_bits"] = Value::Null;
    result["pairing"] = json!({
        "pairing_pending": false,
        "pairing_progress": 0,
        "pairing_required": 0,
        "pending_client_id": "",
        "pending_app_name": "",
        "button_sequence": []
    });

    let mut client_id_seen = 0u8;
    for tlv in frame.tlvs.as_deref().unwrap_or(&[]) {
        match tlv.tlv_type {
            value if value == TlvType::ProtocolVersion as u16 => {
                result["protocol_version"] = json!(tlv_value_u16(tlv)?)
            }
            value if value == TlvType::MaxFrame as u16 => {
                result["max_frame"] = json!(tlv_value_u16(tlv)?)
            }
            value if value == TlvType::Mtu as u16 => result["mtu"] = json!(tlv_value_u16(tlv)?),
            value if value == TlvType::MaxPayload as u16 => {
                result["max_payload"] = json!(tlv_value_u16(tlv)?)
            }
            value if value == TlvType::FeatureBits as u16 => {
                result["feature_bits"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::ClientId as u16 => {
                client_id_seen += 1;
                if client_id_seen == 1 {
                    result["client_id"] = json!(tlv_value_string(tlv));
                } else {
                    result["pairing"]["pending_client_id"] = json!(tlv_value_string(tlv));
                }
            }
            value if value == TlvType::PairingPending as u16 => {
                result["pairing"]["pairing_pending"] = json!(tlv_value_bool(tlv)?)
            }
            value if value == TlvType::PairingProgress as u16 => {
                result["pairing"]["pairing_progress"] = json!(tlv_value_u8(tlv)?)
            }
            value if value == TlvType::PairingRequired as u16 => {
                result["pairing"]["pairing_required"] = json!(tlv_value_u8(tlv)?)
            }
            value if value == TlvType::AppName as u16 => {
                result["pairing"]["pending_app_name"] = json!(tlv_value_string(tlv))
            }
            value if value == TlvType::ButtonSequence as u16 => {
                result["pairing"]["button_sequence"] = json!(button_sequence_names(&tlv.value))
            }
            _ => {}
        }
    }
    Ok(result)
}

pub fn decode_auth_challenge(frame: &Frame) -> Result<Value> {
    let mut result = json!({
        "opcode": opcode_name(frame.opcode),
        "request_id": frame.request_id,
        "client_id": "",
        "nonce_hex": ""
    });
    for tlv in frame.tlvs.as_deref().unwrap_or(&[]) {
        match tlv.tlv_type {
            value if value == TlvType::ClientId as u16 => {
                result["client_id"] = json!(tlv_value_string(tlv))
            }
            value if value == TlvType::AuthNonce as u16 => {
                result["nonce_hex"] = json!(hex::encode(&tlv.value))
            }
            _ => {}
        }
    }
    Ok(result)
}

pub fn decode_trusted_list(frame: &Frame) -> Result<Value> {
    let mut result = json!({
        "opcode": opcode_name(frame.opcode),
        "request_id": frame.request_id,
        "trusted_count": 0,
        "clients": []
    });
    let mut clients = Vec::<Value>::new();
    let mut current = json!({"client_id": "", "app_name": "", "created_at": 0});
    let mut has_current = false;
    for tlv in frame.tlvs.as_deref().unwrap_or(&[]) {
        match tlv.tlv_type {
            value if value == TlvType::TrustedCount as u16 => {
                result["trusted_count"] = json!(tlv_value_u8(tlv)?)
            }
            value if value == TlvType::ClientId as u16 => {
                if has_current {
                    clients.push(current);
                }
                current = json!({
                    "client_id": tlv_value_string(tlv),
                    "app_name": "",
                    "created_at": 0
                });
                has_current = true;
            }
            value if value == TlvType::AppName as u16 && has_current => {
                current["app_name"] = json!(tlv_value_string(tlv))
            }
            value if value == TlvType::CreatedAt as u16 && has_current => {
                current["created_at"] = json!(tlv_value_u32(tlv)?)
            }
            _ => {}
        }
    }
    if has_current {
        clients.push(current);
    }
    result["clients"] = json!(clients);
    Ok(result)
}

pub fn decode_snapshot(frame: &Frame) -> Result<Value> {
    let mut result = json!({
        "opcode": opcode_name(frame.opcode),
        "request_id": frame.request_id,
        "generation": null,
        "uptime_ms": null,
        "auth": {
            "authenticated": false,
            "client_id": "",
            "trusted_client_count": 0
        },
        "pairing": {
            "pairing_pending": false,
            "pairing_progress": 0,
            "pairing_required": 0,
            "pending_client_id": "",
            "pending_app_name": "",
            "button_sequence": []
        },
        "playback": {
            "playing": false,
            "paused": false,
            "cartridge_checksum": null,
            "track_index": null,
            "track_count": null,
            "position_sec": null,
            "started_at": null,
            "duration_sec": null,
            "volume_percent": null,
            "playback_mode": null,
            "track_title": "",
            "track_artist": "",
            "track_file": "",
            "output_target": null
        },
        "cartridge": {
            "status": null,
            "mounted": false,
            "checksum": null,
            "metadata_version": null,
            "track_count": null
        },
        "wifi": {
            "state": null,
            "internet": false,
            "autoreconnect": false,
            "active_slot": null,
            "preferred_slot": null,
            "ip": null
        },
        "lastfm": {
            "has_auth_url": false,
            "has_token": false,
            "has_session": false,
            "busy": false,
            "scrobbling": false,
            "now_playing": false,
            "pending_commands": 0,
            "pending_scrobbles": 0,
            "successful": 0,
            "failed": 0,
            "auth_url": "",
            "username": ""
        },
        "history": {
            "album_count": 0,
            "track_count": 0
        },
        "bluetooth": {
            "a2dp_connected": false,
            "bonded_count": 0
        }
    });

    let mut client_id_seen = 0u8;
    let mut cartridge_checksum_seen = 0u8;
    let mut track_count_seen = 0u8;
    for tlv in frame.tlvs.as_deref().unwrap_or(&[]) {
        match tlv.tlv_type {
            value if value == TlvType::Generation as u16 => {
                result["generation"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::UptimeMs as u16 => {
                result["uptime_ms"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::Authenticated as u16 => {
                result["auth"]["authenticated"] = json!(tlv_value_bool(tlv)?)
            }
            value if value == TlvType::ClientId as u16 => {
                client_id_seen += 1;
                if client_id_seen == 1 {
                    result["auth"]["client_id"] = json!(tlv_value_string(tlv));
                } else {
                    result["pairing"]["pending_client_id"] = json!(tlv_value_string(tlv));
                }
            }
            value if value == TlvType::TrustedCount as u16 => {
                result["auth"]["trusted_client_count"] = json!(tlv_value_u8(tlv)?)
            }
            value if value == TlvType::PairingPending as u16 => {
                result["pairing"]["pairing_pending"] = json!(tlv_value_bool(tlv)?)
            }
            value if value == TlvType::PairingProgress as u16 => {
                result["pairing"]["pairing_progress"] = json!(tlv_value_u8(tlv)?)
            }
            value if value == TlvType::PairingRequired as u16 => {
                result["pairing"]["pairing_required"] = json!(tlv_value_u8(tlv)?)
            }
            value if value == TlvType::AppName as u16 => {
                result["pairing"]["pending_app_name"] = json!(tlv_value_string(tlv))
            }
            value if value == TlvType::ButtonSequence as u16 => {
                result["pairing"]["button_sequence"] = json!(button_sequence_names(&tlv.value))
            }
            value if value == TlvType::Playing as u16 => {
                result["playback"]["playing"] = json!(tlv_value_bool(tlv)?)
            }
            value if value == TlvType::Paused as u16 => {
                result["playback"]["paused"] = json!(tlv_value_bool(tlv)?)
            }
            value if value == TlvType::CartridgeChecksum as u16 => {
                cartridge_checksum_seen += 1;
                if cartridge_checksum_seen == 1 {
                    result["playback"]["cartridge_checksum"] = json!(tlv_value_u32(tlv)?);
                } else {
                    result["cartridge"]["checksum"] = json!(tlv_value_u32(tlv)?);
                }
            }
            value if value == TlvType::TrackIndex as u16 => {
                result["playback"]["track_index"] = json!(ensure_track_index(tlv_value_u32(tlv)?))
            }
            value if value == TlvType::TrackCount as u16 => {
                track_count_seen += 1;
                if track_count_seen == 1 {
                    result["playback"]["track_count"] = json!(tlv_value_u32(tlv)?);
                } else {
                    result["cartridge"]["track_count"] = json!(tlv_value_u32(tlv)?);
                }
            }
            value if value == TlvType::PositionSec as u16 => {
                result["playback"]["position_sec"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::StartedAt as u16 => {
                result["playback"]["started_at"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::DurationSec as u16 => {
                result["playback"]["duration_sec"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::VolumePercent as u16 => {
                result["playback"]["volume_percent"] = json!(tlv_value_u8(tlv)?)
            }
            value if value == TlvType::PlaybackMode as u16 => {
                result["playback"]["playback_mode"] = playback_mode_from_id(tlv_value_u8(tlv)?)
            }
            value if value == TlvType::TrackTitle as u16 => {
                result["playback"]["track_title"] = json!(tlv_value_string(tlv))
            }
            value if value == TlvType::TrackArtist as u16 => {
                result["playback"]["track_artist"] = json!(tlv_value_string(tlv))
            }
            value if value == TlvType::TrackFile as u16 => {
                result["playback"]["track_file"] = json!(tlv_value_string(tlv))
            }
            value if value == TlvType::OutputTarget as u16 => {
                result["playback"]["output_target"] = output_target_from_id(tlv_value_u8(tlv)?)
            }
            value if value == TlvType::CartridgeStatus as u16 => {
                result["cartridge"]["status"] = cartridge_status_from_id(tlv_value_u8(tlv)?)
            }
            value if value == TlvType::CartridgeMounted as u16 => {
                result["cartridge"]["mounted"] = json!(tlv_value_bool(tlv)?)
            }
            value if value == TlvType::MetadataVersion as u16 => {
                result["cartridge"]["metadata_version"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::WifiState as u16 => {
                result["wifi"]["state"] = wifi_state_from_id(tlv_value_u8(tlv)?)
            }
            value if value == TlvType::WifiInternet as u16 => {
                result["wifi"]["internet"] = json!(tlv_value_bool(tlv)?)
            }
            value if value == TlvType::WifiAutoreconnect as u16 => {
                result["wifi"]["autoreconnect"] = json!(tlv_value_bool(tlv)?)
            }
            value if value == TlvType::WifiActiveSlot as u16 => {
                result["wifi"]["active_slot"] = json!(decode_slot(tlv_value_u8(tlv)?))
            }
            value if value == TlvType::WifiPreferredSlot as u16 => {
                result["wifi"]["preferred_slot"] = json!(decode_slot(tlv_value_u8(tlv)?))
            }
            value if value == TlvType::WifiIp as u16 => {
                result["wifi"]["ip"] = json!(decode_ip_address(tlv_value_u32(tlv)?))
            }
            value if value == TlvType::LastfmHasAuthUrl as u16 => {
                result["lastfm"]["has_auth_url"] = json!(tlv_value_bool(tlv)?)
            }
            value if value == TlvType::LastfmHasToken as u16 => {
                result["lastfm"]["has_token"] = json!(tlv_value_bool(tlv)?)
            }
            value if value == TlvType::LastfmHasSession as u16 => {
                result["lastfm"]["has_session"] = json!(tlv_value_bool(tlv)?)
            }
            value if value == TlvType::LastfmBusy as u16 => {
                result["lastfm"]["busy"] = json!(tlv_value_bool(tlv)?)
            }
            value if value == TlvType::LastfmScrobbling as u16 => {
                result["lastfm"]["scrobbling"] = json!(tlv_value_bool(tlv)?)
            }
            value if value == TlvType::LastfmNowPlaying as u16 => {
                result["lastfm"]["now_playing"] = json!(tlv_value_bool(tlv)?)
            }
            value if value == TlvType::LastfmPendingCommands as u16 => {
                result["lastfm"]["pending_commands"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::LastfmPendingScrobbles as u16 => {
                result["lastfm"]["pending_scrobbles"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::LastfmSuccessful as u16 => {
                result["lastfm"]["successful"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::LastfmFailed as u16 => {
                result["lastfm"]["failed"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::LastfmAuthUrl as u16 => {
                result["lastfm"]["auth_url"] = json!(tlv_value_string(tlv))
            }
            value if value == TlvType::LastfmUsername as u16 => {
                result["lastfm"]["username"] = json!(tlv_value_string(tlv))
            }
            value if value == TlvType::HistoryAlbumCount as u16 => {
                result["history"]["album_count"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::HistoryTrackCount as u16 => {
                result["history"]["track_count"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::BtA2dpConnected as u16 => {
                result["bluetooth"]["a2dp_connected"] = json!(tlv_value_bool(tlv)?)
            }
            value if value == TlvType::BtBondedCount as u16 => {
                result["bluetooth"]["bonded_count"] = json!(tlv_value_u32(tlv)?)
            }
            _ => {}
        }
    }
    Ok(result)
}

fn artwork_mime_type(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some("image/png");
    }

    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some("image/jpeg");
    }

    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Some("image/gif");
    }

    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some("image/webp");
    }

    if bytes.starts_with(b"BM") {
        return Some("image/bmp");
    }

    if let Ok(text) = std::str::from_utf8(bytes) {
        let trimmed = text.trim_start_matches('\u{feff}').trim_start();
        if trimmed.starts_with("<svg") || (trimmed.starts_with("<?xml") && trimmed.contains("<svg")) {
            return Some("image/svg+xml");
        }
    }

    None
}

fn artwork_data_url(bytes: &[u8]) -> Option<String> {
    let mime_type = artwork_mime_type(bytes)?;
    Some(format!("data:{mime_type};base64,{}", STANDARD.encode(bytes)))
}

pub fn decode_album(frame: &Frame) -> Result<Value> {
    let mut result = json!({
        "opcode": opcode_name(frame.opcode),
        "request_id": frame.request_id,
        "cartridge": {
            "status": null,
            "mounted": false,
            "checksum": null,
            "metadata_version": null,
            "track_count": null
        },
        "album": {
            "name": "",
            "artist": "",
            "description": "",
            "year": null,
            "duration_sec": null,
            "genre": "",
            "artwork_data_url": null
        }
    });
    let mut artwork_bytes = Vec::new();
    for tlv in frame.tlvs.as_deref().unwrap_or(&[]) {
        match tlv.tlv_type {
            value if value == TlvType::CartridgeStatus as u16 => {
                result["cartridge"]["status"] = cartridge_status_from_id(tlv_value_u8(tlv)?)
            }
            value if value == TlvType::CartridgeMounted as u16 => {
                result["cartridge"]["mounted"] = json!(tlv_value_bool(tlv)?)
            }
            value if value == TlvType::CartridgeChecksum as u16 => {
                result["cartridge"]["checksum"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::MetadataVersion as u16 => {
                result["cartridge"]["metadata_version"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::TrackCount as u16 => {
                result["cartridge"]["track_count"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::AlbumName as u16 => {
                result["album"]["name"] = json!(tlv_value_string(tlv))
            }
            value if value == TlvType::AlbumArtist as u16 => {
                result["album"]["artist"] = json!(tlv_value_string(tlv))
            }
            value if value == TlvType::AlbumDescription as u16 => {
                result["album"]["description"] = json!(tlv_value_string(tlv))
            }
            value if value == TlvType::AlbumYear as u16 => {
                result["album"]["year"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::AlbumDuration as u16 => {
                result["album"]["duration_sec"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::AlbumGenre as u16 => {
                result["album"]["genre"] = json!(tlv_value_string(tlv))
            }
            value if value == TlvType::BinaryData as u16 => artwork_bytes.extend_from_slice(&tlv.value),
            _ => {}
        }
    }
    if let Some(data_url) = artwork_data_url(&artwork_bytes) {
        result["album"]["artwork_data_url"] = json!(data_url);
    }
    Ok(result)
}

pub fn decode_track_page(frame: &Frame) -> Result<Value> {
    let mut result = json!({
        "opcode": opcode_name(frame.opcode),
        "request_id": frame.request_id,
        "offset": 0,
        "track_count": 0,
        "returned_count": 0,
        "tracks": []
    });
    let mut tracks = Vec::<Value>::new();
    let mut current = json!({
        "track_index": 0,
        "title": "",
        "artist": "",
        "duration_sec": null,
        "file_num": null
    });
    let mut has_current = false;
    for tlv in frame.tlvs.as_deref().unwrap_or(&[]) {
        match tlv.tlv_type {
            value if value == TlvType::Offset as u16 => {
                result["offset"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::TrackCount as u16 => {
                result["track_count"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::TrackIndex as u16 => {
                if has_current {
                    tracks.push(current);
                }
                current = json!({
                    "track_index": tlv_value_u32(tlv)?,
                    "title": "",
                    "artist": "",
                    "duration_sec": null,
                    "file_num": null
                });
                has_current = true;
            }
            value if value == TlvType::TrackTitle as u16 && has_current => {
                current["title"] = json!(tlv_value_string(tlv))
            }
            value if value == TlvType::TrackArtist as u16 && has_current => {
                current["artist"] = json!(tlv_value_string(tlv))
            }
            value if value == TlvType::DurationSec as u16 && has_current => {
                current["duration_sec"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::TrackFile as u16 && has_current => {
                current["file_num"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::ReturnedCount as u16 => {
                result["returned_count"] = json!(tlv_value_u32(tlv)?)
            }
            _ => {}
        }
    }
    if has_current {
        tracks.push(current);
    }
    result["tracks"] = json!(tracks);
    Ok(result)
}

pub fn decode_wifi_scan_results(frame: &Frame) -> Result<Value> {
    let mut result = json!({
        "opcode": opcode_name(frame.opcode),
        "request_id": frame.request_id,
        "offset": 0,
        "total_count": 0,
        "returned_count": 0,
        "results": []
    });
    let mut items = Vec::<Value>::new();
    let mut current = json!({
        "ssid": "",
        "rssi": null,
        "channel": null,
        "authmode": null
    });
    let mut has_current = false;
    for tlv in frame.tlvs.as_deref().unwrap_or(&[]) {
        match tlv.tlv_type {
            value if value == TlvType::Offset as u16 => {
                result["offset"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::Count as u16 => {
                result["total_count"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::WifiSsid as u16 => {
                if has_current {
                    items.push(current);
                }
                current = json!({
                    "ssid": tlv_value_string(tlv),
                    "rssi": null,
                    "channel": null,
                    "authmode": null
                });
                has_current = true;
            }
            value if value == TlvType::WifiRssi as u16 && has_current => {
                current["rssi"] = json!(read_i32_from_u32(tlv_value_u32(tlv)?))
            }
            value if value == TlvType::WifiChannel as u16 && has_current => {
                current["channel"] = json!(tlv_value_u8(tlv)?)
            }
            value if value == TlvType::WifiAuthMode as u16 && has_current => {
                current["authmode"] = json!(tlv_value_u8(tlv)?)
            }
            value if value == TlvType::ReturnedCount as u16 => {
                result["returned_count"] = json!(tlv_value_u32(tlv)?)
            }
            _ => {}
        }
    }
    if has_current {
        items.push(current);
    }
    result["results"] = json!(items);
    Ok(result)
}

pub fn decode_history_album_page(frame: &Frame) -> Result<Value> {
    let mut result = json!({
        "opcode": opcode_name(frame.opcode),
        "request_id": frame.request_id,
        "offset": 0,
        "album_count": 0,
        "returned_count": 0,
        "albums": []
    });
    let mut items = Vec::<Value>::new();
    let mut current = json!({
        "checksum": 0,
        "track_count": null,
        "first_seen_sequence": null,
        "last_seen_sequence": null,
        "album_name": "",
        "album_artist": ""
    });
    let mut has_current = false;
    for tlv in frame.tlvs.as_deref().unwrap_or(&[]) {
        match tlv.tlv_type {
            value if value == TlvType::Offset as u16 => {
                result["offset"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::HistoryAlbumCount as u16 => {
                result["album_count"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::CartridgeChecksum as u16 => {
                if has_current {
                    items.push(current);
                }
                current = json!({
                    "checksum": tlv_value_u32(tlv)?,
                    "track_count": null,
                    "first_seen_sequence": null,
                    "last_seen_sequence": null,
                    "album_name": "",
                    "album_artist": ""
                });
                has_current = true;
            }
            value if value == TlvType::TrackCount as u16 && has_current => {
                current["track_count"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::HistoryFirstSeen as u16 && has_current => {
                current["first_seen_sequence"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::HistoryLastSeen as u16 && has_current => {
                current["last_seen_sequence"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::AlbumName as u16 && has_current => {
                current["album_name"] = json!(tlv_value_string(tlv))
            }
            value if value == TlvType::AlbumArtist as u16 && has_current => {
                current["album_artist"] = json!(tlv_value_string(tlv))
            }
            value if value == TlvType::ReturnedCount as u16 => {
                result["returned_count"] = json!(tlv_value_u32(tlv)?)
            }
            _ => {}
        }
    }
    if has_current {
        items.push(current);
    }
    result["albums"] = json!(items);
    Ok(result)
}

pub fn decode_heartbeat(frame: &Frame) -> Result<Value> {
    let mut result = json!({
        "opcode": opcode_name(frame.opcode),
        "frame_type": frame_type_name(frame.frame_type),
        "request_id": frame.request_id,
        "uptime_ms": null,
        "generation": null,
        "authenticated": false,
        "queue_free": null,
        "rx_frames": null,
        "tx_frames": null,
        "rx_errors": null
    });
    for tlv in frame.tlvs.as_deref().unwrap_or(&[]) {
        match tlv.tlv_type {
            value if value == TlvType::UptimeMs as u16 => {
                result["uptime_ms"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::Generation as u16 => {
                result["generation"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::Authenticated as u16 => {
                result["authenticated"] = json!(tlv_value_bool(tlv)?)
            }
            value if value == TlvType::QueueFree as u16 => {
                result["queue_free"] = json!(tlv_value_u8(tlv)?)
            }
            value if value == TlvType::RxFrames as u16 => {
                result["rx_frames"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::TxFrames as u16 => {
                result["tx_frames"] = json!(tlv_value_u32(tlv)?)
            }
            value if value == TlvType::RxErrors as u16 => {
                result["rx_errors"] = json!(tlv_value_u32(tlv)?)
            }
            _ => {}
        }
    }
    Ok(result)
}

pub fn decode_generic_tlvs(frame: &Frame) -> Result<Value> {
    let mut items = Vec::<Value>::new();
    for tlv in frame.tlvs.as_deref().unwrap_or(&[]) {
        let value = if tlv.value.len() == 1 {
            json!(tlv.value[0])
        } else if tlv.value.len() == 2 {
            json!(read_u16(&tlv.value)?)
        } else if tlv.value.len() == 4 {
            json!(read_u32(&tlv.value)?)
        } else if let Ok(text) = String::from_utf8(tlv.value.clone()) {
            json!(text)
        } else {
            json!({"value_hex": hex::encode(&tlv.value)})
        };
        items.push(json!({
            "type": tlv.name(),
            "type_id": tlv.tlv_type,
            "value": value
        }));
    }
    Ok(json!({
        "opcode": opcode_name(frame.opcode),
        "frame_type": frame_type_name(frame.frame_type),
        "request_id": frame.request_id,
        "tlvs": items
    }))
}

pub fn decode_frame(frame: &Frame) -> Result<Value> {
    if frame.frame_type == FrameType::Heartbeat as u8 {
        return decode_heartbeat(frame);
    }
    if frame.frame_type == FrameType::Event as u8 && frame.opcode == Opcode::PairStatus as u16 {
        let mut decoded = decode_pair_status(frame)?;
        decoded["frame_type"] = json!(frame_type_name(frame.frame_type));
        return Ok(decoded);
    }
    if frame.frame_type == FrameType::Event as u8
        && matches!(
            frame.opcode,
            value if value == Opcode::Snapshot as u16
                || value == Opcode::PlaybackStatus as u16
                || value == Opcode::PlaybackControl as u16
                || value == Opcode::WifiStatus as u16
                || value == Opcode::WifiScanStart as u16
                || value == Opcode::WifiConnect as u16
                || value == Opcode::WifiConnectSlot as u16
                || value == Opcode::WifiDisconnect as u16
                || value == Opcode::WifiAutoreconnect as u16
                || value == Opcode::LastfmStatus as u16
                || value == Opcode::LastfmControl as u16
                || value == Opcode::HistorySummary as u16
                || value == Opcode::BtAudioStatus as u16
                || value == Opcode::BtAudioControl as u16
        )
    {
        let mut decoded = decode_snapshot(frame)?;
        decoded["frame_type"] = json!(frame_type_name(frame.frame_type));
        return Ok(decoded);
    }
    if frame.frame_type == FrameType::Event as u8 {
        let mut decoded = decode_generic_tlvs(frame)?;
        decoded["frame_type"] = json!(frame_type_name(frame.frame_type));
        return Ok(decoded);
    }
    decode_generic_tlvs(frame)
}

pub fn playback_action_from_request(action: &str) -> Result<PlaybackAction> {
    match action {
        "next" => Ok(PlaybackAction::Next),
        "previous" | "prev" => Ok(PlaybackAction::Previous),
        "pause_toggle" | "pause" => Ok(PlaybackAction::PauseToggle),
        "fast_forward" | "ff" => Ok(PlaybackAction::FastForward),
        "rewind" => Ok(PlaybackAction::Rewind),
        "play_index" => Ok(PlaybackAction::PlayIndex),
        "seek_seconds" | "seek" => Ok(PlaybackAction::SeekSeconds),
        "set_volume_percent" | "volume" => Ok(PlaybackAction::SetVolumePercent),
        "set_mode" | "mode" => Ok(PlaybackAction::SetMode),
        "set_output_target" | "output" => Ok(PlaybackAction::SetOutputTarget),
        _ => Err(CompanionError::UnknownPlaybackAction(action.to_string())),
    }
}

pub fn lastfm_action_from_request(action: &str) -> Result<LastfmAction> {
    match action {
        "set_auth_url" => Ok(LastfmAction::SetAuthUrl),
        "request_token" | "token" => Ok(LastfmAction::RequestToken),
        "auth" => Ok(LastfmAction::Auth),
        "logout" => Ok(LastfmAction::Logout),
        "set_scrobbling" | "scrobble" => Ok(LastfmAction::SetScrobbling),
        "set_now_playing" | "now_playing" => Ok(LastfmAction::SetNowPlaying),
        _ => Err(CompanionError::UnknownLastfmAction(action.to_string())),
    }
}

pub fn bt_action_from_request(action: &str) -> Result<BtAction> {
    match action {
        "connect_last" | "connect-last" => Ok(BtAction::ConnectLast),
        "pair_best" | "pair-best" => Ok(BtAction::PairBest),
        "disconnect" => Ok(BtAction::Disconnect),
        _ => Err(CompanionError::UnknownBluetoothAction(action.to_string())),
    }
}
