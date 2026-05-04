use std::{
    collections::HashMap,
    ffi::c_void,
    sync::{
        atomic::{AtomicI64, Ordering},
        Mutex, OnceLock,
    },
    time::Duration,
};

use jni019::{
    objects::{GlobalRef, JObject, JString, JValue},
    sys::{jbyteArray, jlong, jobject, jstring},
    JNIEnv, JavaVM, NativeMethod,
};
use serde::Deserialize;
use tokio::{sync::mpsc, task};

use crate::companion::{
    error::{CompanionError, Result},
    protocol::{notify_uuid, service_uuid, write_uuid, DiscoveredDevice},
};

static ANDROID_BLE_BRIDGE_INIT_RESULT: OnceLock<std::result::Result<(), String>> = OnceLock::new();
static ANDROID_BLE_BRIDGE_VM: OnceLock<JavaVM> = OnceLock::new();
static ANDROID_BLE_BRIDGE: OnceLock<GlobalRef> = OnceLock::new();
static ANDROID_BLE_NOTIFICATION_SENDERS: OnceLock<Mutex<HashMap<i64, mpsc::UnboundedSender<Vec<u8>>>>> =
    OnceLock::new();
static NEXT_ANDROID_BLE_SESSION_ID: AtomicI64 = AtomicI64::new(1);

const BRIDGE_CLASS: &str = "com/grieferpig/jukeboy_companion/CompanionBleBridge";

#[derive(Debug)]
pub(crate) struct AndroidBleConnection {
    session_id: i64,
    name: String,
}

impl AndroidBleConnection {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    fn session_id(&self) -> i64 {
        self.session_id
    }
}

#[derive(Deserialize)]
struct AndroidConnectResult {
    name: String,
}

fn notification_senders() -> &'static Mutex<HashMap<i64, mpsc::UnboundedSender<Vec<u8>>>> {
    ANDROID_BLE_NOTIFICATION_SENDERS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub(crate) fn init(env: &JNIEnv, activity: jobject) -> std::result::Result<(), String> {
    if let Some(result) = ANDROID_BLE_BRIDGE_INIT_RESULT.get() {
        return result.clone();
    }

    let init_result = (|| {
        let vm = env
            .get_java_vm()
            .map_err(|error| format!("failed to capture Android JavaVM: {error}"))?;

        env.register_native_methods(
            BRIDGE_CLASS,
            &[
                NativeMethod {
                    name: "nativeOnNotification".into(),
                    sig: "(J[B)V".into(),
                    fn_ptr: native_on_notification as *mut c_void,
                },
                NativeMethod {
                    name: "nativeOnSessionClosed".into(),
                    sig: "(JLjava/lang/String;)V".into(),
                    fn_ptr: native_on_session_closed as *mut c_void,
                },
            ],
        )
        .map_err(|error| format!("failed to register Android BLE bridge native methods: {error}"))?;

        let activity = JObject::from(activity);
        let bridge = env
            .call_static_method(
                BRIDGE_CLASS,
                "getInstance",
                "(Landroid/app/Activity;)Lcom/grieferpig/jukeboy_companion/CompanionBleBridge;",
                &[JValue::Object(activity)],
            )
            .and_then(|value| value.l())
            .map_err(|error| format!("failed to create Android BLE bridge: {error}"))?;
        let bridge = env
            .new_global_ref(bridge)
            .map_err(|error| format!("failed to store Android BLE bridge instance: {error}"))?;

        let _ = ANDROID_BLE_BRIDGE_VM.set(vm);
        let _ = ANDROID_BLE_BRIDGE.set(bridge);
        Ok(())
    })();

    let _ = ANDROID_BLE_BRIDGE_INIT_RESULT.set(init_result.clone());
    init_result
}

pub(crate) async fn scan_devices(scan_timeout: Duration) -> Result<Vec<DiscoveredDevice>> {
    run_blocking(move || scan_devices_blocking(scan_timeout)).await
}

pub(crate) async fn connect(
    address: String,
    timeout_duration: Duration,
) -> Result<(AndroidBleConnection, mpsc::UnboundedReceiver<Vec<u8>>)> {
    run_blocking(move || connect_blocking(address, timeout_duration)).await
}

pub(crate) async fn write_chunk(connection: &AndroidBleConnection, chunk: Vec<u8>) -> Result<()> {
    let session_id = connection.session_id();
    run_blocking(move || write_chunk_blocking(session_id, chunk)).await
}

pub(crate) async fn disconnect(connection: &AndroidBleConnection) -> Result<()> {
    let session_id = connection.session_id();
    run_blocking(move || disconnect_blocking(session_id)).await
}

async fn run_blocking<T: Send + 'static>(
    operation: impl FnOnce() -> Result<T> + Send + 'static,
) -> Result<T> {
    task::spawn_blocking(operation).await.map_err(|error| {
        CompanionError::AndroidBleBridge(format!("Android BLE task failed: {error}"))
    })?
}

fn scan_devices_blocking(scan_timeout: Duration) -> Result<Vec<DiscoveredDevice>> {
    with_bridge_env(|env, bridge| {
        let service_uuid = env
            .new_string(service_uuid().to_string())
            .map_err(|error| CompanionError::AndroidBleBridge(format!("failed to allocate service UUID string: {error}")))?;
        let response = env
            .call_method(
                bridge.as_obj(),
                "scanDevices",
                "(Ljava/lang/String;J)Ljava/lang/String;",
                &[
                    JValue::Object(JObject::from(service_uuid)),
                    JValue::Long(duration_to_millis(scan_timeout) as jlong),
                ],
            )
            .and_then(|value| value.l())
            .map_err(|error| CompanionError::AndroidBleBridge(format!("Android BLE scan failed: {error}")))?;
        parse_json_string(env, response, "scan results")
    })
}

fn connect_blocking(
    address: String,
    timeout_duration: Duration,
) -> Result<(AndroidBleConnection, mpsc::UnboundedReceiver<Vec<u8>>)> {
    let session_id = NEXT_ANDROID_BLE_SESSION_ID.fetch_add(1, Ordering::SeqCst);
    let (notification_tx, notification_rx) = mpsc::unbounded_channel();
    notification_senders()
        .lock()
        .unwrap()
        .insert(session_id, notification_tx);

    let connect_result = with_bridge_env(|env, bridge| {
        let address = env
            .new_string(&address)
            .map_err(|error| CompanionError::AndroidBleBridge(format!("failed to allocate Android BLE address string: {error}")))?;
        let service_uuid = env
            .new_string(service_uuid().to_string())
            .map_err(|error| CompanionError::AndroidBleBridge(format!("failed to allocate service UUID string: {error}")))?;
        let write_uuid = env
            .new_string(write_uuid().to_string())
            .map_err(|error| CompanionError::AndroidBleBridge(format!("failed to allocate write UUID string: {error}")))?;
        let notify_uuid = env
            .new_string(notify_uuid().to_string())
            .map_err(|error| CompanionError::AndroidBleBridge(format!("failed to allocate notify UUID string: {error}")))?;

        let response = env
            .call_method(
                bridge.as_obj(),
                "connectSession",
                "(JLjava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;J)Ljava/lang/String;",
                &[
                    JValue::Long(session_id as jlong),
                    JValue::Object(JObject::from(address)),
                    JValue::Object(JObject::from(service_uuid)),
                    JValue::Object(JObject::from(write_uuid)),
                    JValue::Object(JObject::from(notify_uuid)),
                    JValue::Long(duration_to_millis(timeout_duration) as jlong),
                ],
            )
            .and_then(|value| value.l())
            .map_err(|error| CompanionError::AndroidBleBridge(format!("Android BLE connect failed: {error}")))?;
        parse_json_string::<AndroidConnectResult>(env, response, "connect result")
    });

    match connect_result {
        Ok(connect_result) => Ok((
            AndroidBleConnection {
                session_id,
                name: connect_result.name,
            },
            notification_rx,
        )),
        Err(error) => {
            notification_senders().lock().unwrap().remove(&session_id);
            Err(error)
        }
    }
}

fn write_chunk_blocking(session_id: i64, chunk: Vec<u8>) -> Result<()> {
    with_bridge_env(|env, bridge| {
        let chunk = env
            .byte_array_from_slice(&chunk)
            .map_err(|error| CompanionError::AndroidBleBridge(format!("failed to allocate Android BLE write buffer: {error}")))?;
        env.call_method(
            bridge.as_obj(),
            "writeChunk",
            "(J[B)V",
            &[
                JValue::Long(session_id as jlong),
                JValue::Object(JObject::from(chunk)),
            ],
        )
        .map(|_| ())
        .map_err(|error| CompanionError::AndroidBleBridge(format!("Android BLE write failed: {error}")))
    })
}

fn disconnect_blocking(session_id: i64) -> Result<()> {
    let result = with_bridge_env(|env, bridge| {
        env.call_method(
            bridge.as_obj(),
            "disconnectSession",
            "(J)V",
            &[JValue::Long(session_id as jlong)],
        )
        .map(|_| ())
        .map_err(|error| CompanionError::AndroidBleBridge(format!("Android BLE disconnect failed: {error}")))
    });
    notification_senders().lock().unwrap().remove(&session_id);
    result
}

fn with_bridge_env<T>(operation: impl FnOnce(&JNIEnv, &GlobalRef) -> Result<T>) -> Result<T> {
    ensure_bridge_initialized()?;

    let vm = ANDROID_BLE_BRIDGE_VM.get().ok_or_else(|| {
        CompanionError::AndroidBleBridge(
            "Android BLE bridge did not capture the Java VM at startup".into(),
        )
    })?;
    let bridge = ANDROID_BLE_BRIDGE.get().ok_or_else(|| {
        CompanionError::AndroidBleBridge(
            "Android BLE bridge instance is unavailable".into(),
        )
    })?;
    let env = vm.attach_current_thread_permanently().map_err(|error| {
        CompanionError::AndroidBleBridge(format!(
            "failed to attach current thread to the Android JVM: {error}"
        ))
    })?;

    operation(&env, bridge)
}

fn ensure_bridge_initialized() -> Result<()> {
    match ANDROID_BLE_BRIDGE_INIT_RESULT.get() {
        Some(Ok(())) => Ok(()),
        Some(Err(message)) => Err(CompanionError::AndroidBleBridge(message.clone())),
        None => Err(CompanionError::AndroidBleBridge(
            "Android BLE bridge was not initialized during application startup".into(),
        )),
    }
}

fn parse_json_string<T: for<'de> Deserialize<'de>>(
    env: &JNIEnv,
    value: JObject,
    what: &str,
) -> Result<T> {
    let value: JString = value.into();
    let value: String = env
        .get_string(value)
        .map_err(|error| CompanionError::AndroidBleBridge(format!("failed to read Android BLE {what}: {error}")))?
        .into();
    serde_json::from_str(&value).map_err(|error| {
        CompanionError::AndroidBleBridge(format!("failed to parse Android BLE {what}: {error}"))
    })
}

fn duration_to_millis(duration: Duration) -> i64 {
    duration.as_millis().min(i64::MAX as u128) as i64
}

extern "system" fn native_on_notification(
    env: JNIEnv,
    _: JObject,
    session_id: jlong,
    payload: jbyteArray,
) {
    let bytes = match env.convert_byte_array(payload) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("failed to convert Android BLE notification payload: {error}");
            return;
        }
    };

    if let Some(sender) = notification_senders()
        .lock()
        .unwrap()
        .get(&(session_id as i64))
        .cloned()
    {
        let _ = sender.send(bytes);
    }
}

extern "system" fn native_on_session_closed(
    _env: JNIEnv,
    _: JObject,
    session_id: jlong,
    _reason: jstring,
) {
    notification_senders()
        .lock()
        .unwrap()
        .remove(&(session_id as i64));
}