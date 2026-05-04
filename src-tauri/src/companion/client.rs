use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};

#[cfg(target_os = "android")]
use crate::companion::android_ble::{self, AndroidBleConnection};
#[cfg(not(target_os = "android"))]
use btleplug::{
    api::{
        Central, Characteristic, Manager as _, Peripheral as _, ScanFilter, ValueNotification,
        WriteType,
    },
    platform::{Adapter, Manager, Peripheral},
};
#[cfg(not(target_os = "android"))]
use futures_util::StreamExt;
use serde_json::{json, Value};
use tokio::{
    sync::{broadcast, oneshot, Mutex},
    task::JoinHandle,
    time::timeout,
};

#[cfg(not(target_os = "android"))]
use tokio::time::sleep;

use crate::companion::{
    error::{CompanionError, Result},
    protocol::{
        build_auth_proof, decode_album, decode_auth_challenge, decode_auth_status,
        decode_capabilities, decode_frame, decode_frame_bytes, decode_hello,
        decode_history_album_page, decode_pair_status, decode_snapshot, decode_track_page,
        decode_trusted_list, decode_wifi_scan_results, encode_request_frame, opcode_name,
        read_u16, tlv_bytes, tlv_first, tlv_string, tlv_u32, tlv_u8, tlv_value_string,
        tlv_value_u16, BtAction, ConnectedDevice, DiscoveredDevice, Frame, FrameType,
        LastfmAction, Opcode, PlaybackAction, TlvType, DEFAULT_CHUNK_SIZE, DEFAULT_DEVICE_NAME,
        FRAME_HEADER_LEN, FRAME_MAX_LEN, MAGIC, VERSION,
    },
};

    #[cfg(not(target_os = "android"))]
    use crate::companion::protocol::{notify_uuid, service_uuid, write_uuid};

#[derive(Debug)]
struct ClientInner {
    pending: Mutex<HashMap<u32, oneshot::Sender<Result<Frame>>>>,
    rx_buffer: Mutex<Vec<u8>>,
    event_tx: broadcast::Sender<Value>,
}

pub struct CompanionBleClient {
    #[cfg(not(target_os = "android"))]
    peripheral: Peripheral,
    #[cfg(not(target_os = "android"))]
    write_char: Characteristic,
    #[cfg(not(target_os = "android"))]
    notify_char: Characteristic,
    #[cfg(target_os = "android")]
    connection: AndroidBleConnection,
    max_chunk_size: usize,
    timeout: Duration,
    next_request_id: AtomicU32,
    inner: Arc<ClientInner>,
    notification_task: Option<JoinHandle<()>>,
    connected_device: ConnectedDevice,
}

impl CompanionBleClient {
    pub async fn scan(scan_timeout: Duration) -> Result<Vec<DiscoveredDevice>> {
        #[cfg(target_os = "android")]
        {
            let mut results = android_ble::scan_devices(scan_timeout).await?;
            results.retain(|device| device.service_match);
            results.sort_by(|left, right| left.address.cmp(&right.address));
            return Ok(results);
        }

        #[cfg(not(target_os = "android"))]
        {
        let adapter = first_adapter().await?;
        adapter.start_scan(ScanFilter::default()).await?;
        sleep(scan_timeout).await;

        let peripherals = adapter.peripherals().await?;
        let service_uuid = service_uuid().to_string().to_lowercase();
        let mut results = Vec::new();
        for peripheral in peripherals {
            if let Some(properties) = peripheral.properties().await? {
                let uuids: Vec<String> = properties
                    .services
                    .iter()
                    .map(|value| value.to_string().to_lowercase())
                    .collect();
                let service_match = uuids.iter().any(|uuid| uuid == &service_uuid);

                if !service_match {
                    continue;
                }

                results.push(DiscoveredDevice {
                    address: peripheral.address().to_string(),
                    name: properties.local_name.unwrap_or_default(),
                    service_match,
                    uuids,
                });
            }
        }

        results.sort_by(|left, right| left.address.cmp(&right.address));
        Ok(results)
        }
    }

    pub async fn connect(
        address: Option<&str>,
        name: Option<&str>,
        profile: &str,
        scan_timeout: Duration,
        timeout_duration: Duration,
    ) -> Result<Self> {
        #[cfg(target_os = "android")]
        {
            let discovered = android_ble::scan_devices(scan_timeout).await?;
            let selected = resolve_android_device(discovered, address, name)?;

            let (connection, mut notifications) =
                android_ble::connect(selected.address.clone(), timeout_duration).await?;

            let (event_tx, _) = broadcast::channel(64);
            let inner = Arc::new(ClientInner {
                pending: Mutex::new(HashMap::new()),
                rx_buffer: Mutex::new(Vec::new()),
                event_tx,
            });

            let reader_inner = Arc::clone(&inner);
            let notification_task = tokio::spawn(async move {
                while let Some(notification) = notifications.recv().await {
                    if process_notification_bytes(Arc::clone(&reader_inner), &notification)
                        .await
                        .is_err()
                    {
                        continue;
                    }
                }

                fail_pending(
                    &reader_inner,
                    CompanionError::Protocol("notification stream ended".into()),
                )
                .await;
            });

            let device_name = if connection.name().is_empty() {
                if selected.name.is_empty() {
                    DEFAULT_DEVICE_NAME.to_string()
                } else {
                    selected.name.clone()
                }
            } else {
                connection.name().to_string()
            };

            return Ok(Self {
                connection,
                max_chunk_size: DEFAULT_CHUNK_SIZE,
                timeout: timeout_duration,
                next_request_id: AtomicU32::new(1),
                inner,
                notification_task: Some(notification_task),
                connected_device: ConnectedDevice {
                    address: selected.address,
                    name: device_name,
                    profile: profile.to_string(),
                },
            });
        }

        #[cfg(not(target_os = "android"))]
        {
        let adapter = first_adapter().await?;
        adapter.start_scan(ScanFilter::default()).await?;
        sleep(scan_timeout).await;

        let peripheral = resolve_device(&adapter, address, name).await?;
        peripheral.connect().await?;
        peripheral.discover_services().await?;

        let characteristics = peripheral.characteristics();
        let write_char = characteristics
            .iter()
            .find(|characteristic| characteristic.uuid == write_uuid())
            .cloned()
            .ok_or_else(|| CompanionError::Protocol("write characteristic not found".into()))?;
        let notify_char = characteristics
            .iter()
            .find(|characteristic| characteristic.uuid == notify_uuid())
            .cloned()
            .ok_or_else(|| CompanionError::Protocol("notify characteristic not found".into()))?;

        let (event_tx, _) = broadcast::channel(64);
        let inner = Arc::new(ClientInner {
            pending: Mutex::new(HashMap::new()),
            rx_buffer: Mutex::new(Vec::new()),
            event_tx,
        });

        peripheral.subscribe(&notify_char).await?;
        let mut notifications = peripheral.notifications().await?;
        let reader_inner = Arc::clone(&inner);
        let notification_task = tokio::spawn(async move {
            while let Some(notification) = notifications.next().await {
                if process_notification(Arc::clone(&reader_inner), notification)
                    .await
                    .is_err()
                {
                    continue;
                }
            }
            fail_pending(
                &reader_inner,
                CompanionError::Protocol("notification stream ended".into()),
            )
            .await;
        });

        let connected_device = ConnectedDevice {
            address: peripheral.address().to_string(),
            name: peripheral
                .properties()
                .await?
                .and_then(|properties| properties.local_name)
                .unwrap_or_else(|| DEFAULT_DEVICE_NAME.to_string()),
            profile: profile.to_string(),
        };

        Ok(Self {
            peripheral,
            write_char,
            notify_char,
            max_chunk_size: DEFAULT_CHUNK_SIZE,
            timeout: timeout_duration,
            next_request_id: AtomicU32::new(1),
            inner,
            notification_task: Some(notification_task),
            connected_device,
        })
        }
    }

    pub fn connected_device(&self) -> &ConnectedDevice {
        &self.connected_device
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<Value> {
        self.inner.event_tx.subscribe()
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(task) = self.notification_task.take() {
            task.abort();
        }

        #[cfg(target_os = "android")]
        {
            let _ = android_ble::disconnect(&self.connection).await;
        }

        #[cfg(not(target_os = "android"))]
        {
        let _ = self.peripheral.unsubscribe(&self.notify_char).await;
        let _ = self.peripheral.disconnect().await;
        }

        fail_pending(&self.inner, CompanionError::NotConnected).await;
        Ok(())
    }

    pub async fn hello(&self) -> Result<Value> {
        decode_hello(&self.request(Opcode::Hello as u16, None, None).await?)
    }

    pub async fn capabilities(&self) -> Result<Value> {
        decode_capabilities(
            &self
                .request(Opcode::Capabilities as u16, None, None)
                .await?,
        )
    }

    pub async fn ping(&self, text: &str) -> Result<Value> {
        let frame = self
            .request(Opcode::Ping as u16, None, Some(text.as_bytes().to_vec()))
            .await?;
        Ok(json!({
            "opcode": opcode_name(frame.opcode),
            "request_id": frame.request_id,
            "echo": String::from_utf8_lossy(&frame.payload),
            "echo_hex": hex::encode(&frame.payload)
        }))
    }

    pub async fn pair_begin(
        &self,
        client_id: &str,
        app_name: &str,
        secret: &[u8],
        sequence: &[u8],
    ) -> Result<Value> {
        decode_pair_status(
            &self
                .request(
                    Opcode::PairBegin as u16,
                    Some(vec![
                        tlv_string(TlvType::ClientId as u16, client_id),
                        tlv_string(TlvType::AppName as u16, app_name),
                        tlv_bytes(TlvType::SharedSecret as u16, secret),
                        tlv_bytes(TlvType::ButtonSequence as u16, sequence),
                    ]),
                    None,
                )
                .await?,
        )
    }

    pub async fn pair_status(&self) -> Result<Value> {
        decode_pair_status(&self.request(Opcode::PairStatus as u16, None, None).await?)
    }

    pub async fn pair_cancel(&self) -> Result<Value> {
        decode_pair_status(&self.request(Opcode::PairCancel as u16, None, None).await?)
    }

    pub async fn auth_challenge(&self, client_id: &str) -> Result<Value> {
        decode_auth_challenge(
            &self
                .request(
                    Opcode::AuthChallenge as u16,
                    Some(vec![tlv_string(TlvType::ClientId as u16, client_id)]),
                    None,
                )
                .await?,
        )
    }

    pub async fn auth_proof(&self, client_id: &str, secret: &[u8], nonce: &[u8]) -> Result<Value> {
        let proof = build_auth_proof(secret, nonce)?;
        decode_auth_status(
            &self
                .request(
                    Opcode::AuthProof as u16,
                    Some(vec![
                        tlv_string(TlvType::ClientId as u16, client_id),
                        tlv_bytes(TlvType::AuthHmac as u16, &proof),
                    ]),
                    None,
                )
                .await?,
        )
    }

    pub async fn trusted_list(&self) -> Result<Value> {
        decode_trusted_list(&self.request(Opcode::TrustedList as u16, None, None).await?)
    }

    pub async fn trusted_revoke(&self, client_id: &str) -> Result<Value> {
        decode_trusted_list(
            &self
                .request(
                    Opcode::TrustedRevoke as u16,
                    Some(vec![tlv_string(TlvType::ClientId as u16, client_id)]),
                    None,
                )
                .await?,
        )
    }

    pub async fn snapshot(&self) -> Result<Value> {
        decode_snapshot(&self.request(Opcode::Snapshot as u16, None, None).await?)
    }

    pub async fn playback_status(&self) -> Result<Value> {
        decode_snapshot(
            &self
                .request(Opcode::PlaybackStatus as u16, None, None)
                .await?,
        )
    }

    pub async fn playback_control(
        &self,
        action: PlaybackAction,
        value: Option<u32>,
    ) -> Result<Value> {
        let mut tlvs = vec![tlv_u8(TlvType::Action as u16, action as u8)];
        if let Some(value) = value {
            tlvs.push(tlv_u32(TlvType::Value as u16, value));
        }
        decode_snapshot(
            &self
                .request(Opcode::PlaybackControl as u16, Some(tlvs), None)
                .await?,
        )
    }

    pub async fn library_album(&self) -> Result<Value> {
        decode_album(
            &self
                .request(Opcode::LibraryAlbum as u16, None, None)
                .await?,
        )
    }

    pub async fn library_track_page(&self, offset: u32, count: u32) -> Result<Value> {
        decode_track_page(
            &self
                .request(
                    Opcode::LibraryTrackPage as u16,
                    Some(vec![
                        tlv_u32(TlvType::Offset as u16, offset),
                        tlv_u32(TlvType::Count as u16, count),
                    ]),
                    None,
                )
                .await?,
        )
    }

    pub async fn wifi_status(&self) -> Result<Value> {
        decode_snapshot(&self.request(Opcode::WifiStatus as u16, None, None).await?)
    }

    pub async fn wifi_scan_start(&self) -> Result<Value> {
        decode_snapshot(
            &self
                .request(Opcode::WifiScanStart as u16, None, None)
                .await?,
        )
    }

    pub async fn wifi_scan_results(&self, offset: u32, count: u32) -> Result<Value> {
        decode_wifi_scan_results(
            &self
                .request(
                    Opcode::WifiScanResults as u16,
                    Some(vec![
                        tlv_u32(TlvType::Offset as u16, offset),
                        tlv_u32(TlvType::Count as u16, count),
                    ]),
                    None,
                )
                .await?,
        )
    }

    pub async fn wifi_connect(&self, ssid: &str, password: &str) -> Result<Value> {
        decode_snapshot(
            &self
                .request(
                    Opcode::WifiConnect as u16,
                    Some(vec![
                        tlv_string(TlvType::WifiSsid as u16, ssid),
                        tlv_string(TlvType::WifiPassword as u16, password),
                    ]),
                    None,
                )
                .await?,
        )
    }

    pub async fn wifi_connect_slot(&self, slot: u32) -> Result<Value> {
        decode_snapshot(
            &self
                .request(
                    Opcode::WifiConnectSlot as u16,
                    Some(vec![tlv_u32(TlvType::WifiSlot as u16, slot)]),
                    None,
                )
                .await?,
        )
    }

    pub async fn wifi_disconnect(&self) -> Result<Value> {
        decode_snapshot(
            &self
                .request(Opcode::WifiDisconnect as u16, None, None)
                .await?,
        )
    }

    pub async fn wifi_autoreconnect(&self, enabled: bool) -> Result<Value> {
        decode_snapshot(
            &self
                .request(
                    Opcode::WifiAutoreconnect as u16,
                    Some(vec![tlv_u32(TlvType::Value as u16, u32::from(enabled))]),
                    None,
                )
                .await?,
        )
    }

    pub async fn lastfm_status(&self) -> Result<Value> {
        decode_snapshot(
            &self
                .request(Opcode::LastfmStatus as u16, None, None)
                .await?,
        )
    }

    pub async fn lastfm_control(
        &self,
        action: LastfmAction,
        auth_url: Option<&str>,
        username: Option<&str>,
        password: Option<&str>,
        enabled: Option<bool>,
    ) -> Result<Value> {
        let mut tlvs = vec![tlv_u8(TlvType::Action as u16, action as u8)];
        if let Some(auth_url) = auth_url {
            tlvs.push(tlv_string(TlvType::LastfmAuthUrl as u16, auth_url));
        }
        if let Some(username) = username {
            tlvs.push(tlv_string(TlvType::LastfmUsername as u16, username));
        }
        if let Some(password) = password {
            tlvs.push(tlv_string(TlvType::WifiPassword as u16, password));
        }
        if let Some(enabled) = enabled {
            tlvs.push(tlv_u32(TlvType::Value as u16, u32::from(enabled)));
        }
        decode_snapshot(
            &self
                .request(Opcode::LastfmControl as u16, Some(tlvs), None)
                .await?,
        )
    }

    pub async fn history_summary(&self) -> Result<Value> {
        decode_snapshot(
            &self
                .request(Opcode::HistorySummary as u16, None, None)
                .await?,
        )
    }

    pub async fn history_album_page(&self, offset: u32, count: u32) -> Result<Value> {
        decode_history_album_page(
            &self
                .request(
                    Opcode::HistoryAlbumPage as u16,
                    Some(vec![
                        tlv_u32(TlvType::Offset as u16, offset),
                        tlv_u32(TlvType::Count as u16, count),
                    ]),
                    None,
                )
                .await?,
        )
    }

    pub async fn bt_audio_status(&self) -> Result<Value> {
        decode_snapshot(
            &self
                .request(Opcode::BtAudioStatus as u16, None, None)
                .await?,
        )
    }

    pub async fn bt_audio_control(&self, action: BtAction) -> Result<Value> {
        decode_snapshot(
            &self
                .request(
                    Opcode::BtAudioControl as u16,
                    Some(vec![tlv_u8(TlvType::Action as u16, action as u8)]),
                    None,
                )
                .await?,
        )
    }

    async fn request(
        &self,
        opcode: u16,
        tlvs: Option<Vec<Vec<u8>>>,
        payload: Option<Vec<u8>>,
    ) -> Result<Frame> {
        let request_id = next_request_id(&self.next_request_id);
        let payload = payload.unwrap_or_else(|| tlvs.unwrap_or_default().concat());
        let frame = encode_request_frame(opcode, request_id, &payload);

        let (sender, receiver) = oneshot::channel();
        self.inner.pending.lock().await.insert(request_id, sender);
        if let Err(error) = self.write(&frame).await {
            self.inner.pending.lock().await.remove(&request_id);
            return Err(error);
        }

        match timeout(self.timeout, receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(CompanionError::NotConnected),
            Err(_) => {
                self.inner.pending.lock().await.remove(&request_id);
                Err(CompanionError::Timeout)
            }
        }
    }

    async fn write(&self, data: &[u8]) -> Result<()> {
        for chunk in data.chunks(self.max_chunk_size) {
            #[cfg(target_os = "android")]
            android_ble::write_chunk(&self.connection, chunk.to_vec()).await?;

            #[cfg(not(target_os = "android"))]
            self.peripheral
                .write(&self.write_char, chunk, WriteType::WithoutResponse)
                .await?;
        }
        Ok(())
    }
}

#[cfg(not(target_os = "android"))]
async fn first_adapter() -> Result<Adapter> {
    let manager = Manager::new().await?;
    manager
        .adapters()
        .await?
        .into_iter()
        .next()
        .ok_or(CompanionError::NoBluetoothAdapter)
}

#[cfg(not(target_os = "android"))]
async fn resolve_device(
    adapter: &Adapter,
    address: Option<&str>,
    name: Option<&str>,
) -> Result<Peripheral> {
    let mut filtered = Vec::new();
    for peripheral in adapter.peripherals().await? {
        let Some(properties) = peripheral.properties().await? else {
            continue;
        };
        let service_match = properties
            .services
            .iter()
            .any(|uuid| *uuid == service_uuid());
        let current_name = properties.local_name.unwrap_or_default();
        let name_match = current_name == DEFAULT_DEVICE_NAME;
        if let Some(address) = address {
            if peripheral.address().to_string().to_lowercase() != address.to_lowercase() {
                continue;
            }
        }
        if let Some(name) = name {
            if current_name != name {
                continue;
            }
        }
        if address.is_some() || name.is_some() || service_match || name_match {
            filtered.push(peripheral);
        }
    }
    filtered
        .into_iter()
        .next()
        .ok_or(CompanionError::DeviceNotFound)
}

#[cfg(target_os = "android")]
fn resolve_android_device(
    devices: Vec<DiscoveredDevice>,
    address: Option<&str>,
    name: Option<&str>,
) -> Result<DiscoveredDevice> {
    devices
        .into_iter()
        .find(|device| {
            let address_match = address
                .map(|value| device.address.eq_ignore_ascii_case(value))
                .unwrap_or(true);
            let name_match = name.map(|value| device.name == value).unwrap_or(true);
            let default_name_match = device.name == DEFAULT_DEVICE_NAME;

            address_match
                && name_match
                && (address.is_some() || name.is_some() || device.service_match || default_name_match)
        })
        .ok_or(CompanionError::DeviceNotFound)
}

fn next_request_id(counter: &AtomicU32) -> u32 {
    loop {
        let current = counter.load(Ordering::Relaxed);
        let next = if current == u32::MAX { 1 } else { current + 1 };
        if counter
            .compare_exchange(current, next, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            return current.max(1);
        }
    }
}

async fn process_notification_bytes(inner: Arc<ClientInner>, notification: &[u8]) -> Result<()> {
    let mut raw_frames = Vec::<Vec<u8>>::new();
    {
        let mut buffer = inner.rx_buffer.lock().await;
        buffer.extend_from_slice(notification);

        while buffer.len() >= FRAME_HEADER_LEN {
            if buffer[0..2] != MAGIC {
                buffer.remove(0);
                continue;
            }
            if buffer[2] != VERSION {
                let drain = FRAME_HEADER_LEN.min(buffer.len());
                buffer.drain(..drain);
                continue;
            }
            let payload_len = read_u16(&buffer[10..12])? as usize;
            let frame_len = FRAME_HEADER_LEN + payload_len;
            if frame_len > FRAME_MAX_LEN {
                let drain = FRAME_HEADER_LEN.min(buffer.len());
                buffer.drain(..drain);
                continue;
            }
            if buffer.len() < frame_len {
                break;
            }
            raw_frames.push(buffer.drain(..frame_len).collect());
        }
    }

    for raw in raw_frames {
        let frame = decode_frame_bytes(&raw)?;
        let is_response = matches!(
            FrameType::try_from(frame.frame_type)?,
            FrameType::Response | FrameType::Error
        );
        if is_response {
            let sender = inner.pending.lock().await.remove(&frame.request_id);
            if let Some(sender) = sender {
                if FrameType::try_from(frame.frame_type)? == FrameType::Error {
                    let tlvs = frame.tlvs.clone().unwrap_or_default();
                    let error_code = tlv_first(&tlvs, TlvType::ErrorCode as u16)
                        .map(tlv_value_u16)
                        .transpose()?
                        .unwrap_or(u16::MAX) as i32;
                    let error_message = tlv_first(&tlvs, TlvType::ErrorMessage as u16)
                        .map(tlv_value_string)
                        .unwrap_or_else(|| "error".to_string());
                    let _ = sender.send(Err(CompanionError::Api {
                        opcode: frame.opcode,
                        request_id: frame.request_id,
                        error_code,
                        message: error_message,
                    }));
                } else {
                    let _ = sender.send(Ok(frame));
                }
                continue;
            }
        }
        let event = decode_frame(&frame)?;
        let _ = inner.event_tx.send(event);
    }
    Ok(())
}

#[cfg(not(target_os = "android"))]
async fn process_notification(
    inner: Arc<ClientInner>,
    notification: ValueNotification,
) -> Result<()> {
    process_notification_bytes(inner, &notification.value).await
}

async fn fail_pending(inner: &ClientInner, error: CompanionError) {
    let mut pending = inner.pending.lock().await;
    let senders = pending
        .drain()
        .map(|(_, sender)| sender)
        .collect::<Vec<_>>();
    drop(pending);
    for sender in senders {
        let _ = sender.send(Err(CompanionError::Protocol(error.to_string())));
    }
}
