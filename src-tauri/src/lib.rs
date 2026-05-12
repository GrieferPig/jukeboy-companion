mod commands;
pub mod companion;

use commands::{
    companion_auth, companion_bt_bonded_list, companion_bt_control, companion_bt_scan_results,
    companion_bt_scan_start, companion_bt_status, companion_bt_unbond, companion_capabilities,
    companion_connect, companion_connection_status, companion_disconnect, companion_hello,
    companion_hid_led_set, companion_hid_status, companion_history_albums,
    companion_history_clear, companion_history_summary, companion_history_tracks,
    companion_lastfm_control, companion_lastfm_request_token, companion_lastfm_status,
    companion_library_album, companion_library_tracks, companion_output_select,
    companion_output_status, companion_pair_begin, companion_pair_cancel,
    companion_pair_status, companion_ping, companion_playback_control,
    companion_playback_status, companion_scan, companion_script_list,
    companion_script_log, companion_script_run, companion_script_status,
    companion_snapshot, companion_system_reboot, companion_system_reboot_download,
    companion_trusted_list, companion_trusted_revoke, companion_wifi_autoreconnect,
    companion_wifi_connect, companion_wifi_connect_slot, companion_wifi_disconnect,
    companion_wifi_list_slots, companion_wifi_reconnect, companion_wifi_save_slot,
    companion_wifi_scan_results, companion_wifi_scan_start, companion_wifi_status,
};
use companion::AppState;
use tauri::Manager;

#[cfg(target_os = "android")]
mod android {
    use super::run;
    use tauri::wry::{
        self,
        prelude::{ndk::looper::ThreadLooper, GlobalRef, JClass, JNIEnv, JString},
    };

    fn pending_exception_to_string(env: &mut JNIEnv) -> Option<String> {
        if !env.exception_check().ok()? {
            return None;
        }

        let throwable = env.exception_occurred().ok()?;
        let _ = env.exception_clear();
        let description = env
            .call_method(&throwable, "toString", "()Ljava/lang/String;", &[])
            .ok()?
            .l()
            .ok()?;
        let description: JString = description.into();
        let description = env
            .get_string(&description)
            .ok()?
            .to_string_lossy()
            .into_owned();

        Some(description)
    }

    unsafe fn android_setup_with_btleplug(
        package: &str,
        mut env: JNIEnv,
        looper: &ThreadLooper,
        activity: GlobalRef,
    ) {
        let init_result = (|| {
            let bridge_env =
                unsafe { jni019::JNIEnv::from_raw(env.get_raw().cast()) }.map_err(|error| {
                    format!("failed to wrap Android JNIEnv for the BLE bridge: {error}")
                })?;

            crate::companion::android_ble::init(&bridge_env, activity.as_obj().as_raw().cast())
                .map_err(|error| {
                    pending_exception_to_string(&mut env)
                        .map(|exception| format!("{error}: {exception}"))
                        .unwrap_or(error)
                })
        })();

        if let Err(error) = &init_result {
            eprintln!("Android BLE bridge init failed: {error}");
        }

        wry::android_setup(package, env, looper, activity);
    }

    fn stop_unwind<F: FnOnce() -> T, T>(f: F) -> T {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("attempt to unwind out of `rust` with err: {:?}", err);
                std::process::abort()
            }
        }
    }

    fn _start_app() {
        tauri::wry::android_binding!(com_grieferpig, jukeboy_companion, ::tauri::wry);
        tauri::tao::android_binding!(
            com_grieferpig,
            jukeboy_companion,
            Rust,
            android_setup_with_btleplug,
            _start_app,
            ::tauri::tao
        );

        tauri::tao::platform::android::prelude::android_fn!(
            app_tauri,
            plugin,
            PluginManager,
            handlePluginResponse,
            [i32, JString, JString],
        );
        tauri::tao::platform::android::prelude::android_fn!(
            app_tauri,
            plugin,
            PluginManager,
            sendChannelData,
            [i64, JString],
        );

        #[allow(non_snake_case)]
        pub fn handlePluginResponse(
            mut env: JNIEnv,
            _: JClass,
            id: i32,
            success: JString,
            error: JString,
        ) {
            tauri::handle_android_plugin_response(&mut env, id, success, error);
        }

        #[allow(non_snake_case)]
        pub fn sendChannelData(mut env: JNIEnv, _: JClass, id: i64, data: JString) {
            tauri::send_channel_data(&mut env, id, data);
        }

        stop_unwind(run);
    }
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {name}! You've been greeted from Rust!")
}

#[cfg_attr(all(mobile, not(target_os = "android")), tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_handle = app.handle().clone();
            let state = AppState::for_app(&app_handle)?;
            state.spawn_event_bridge(app_handle);
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            greet,
            companion_scan,
            companion_connect,
            companion_disconnect,
            companion_connection_status,
            companion_hello,
            companion_capabilities,
            companion_ping,
            companion_pair_begin,
            companion_pair_status,
            companion_pair_cancel,
            companion_auth,
            companion_trusted_list,
            companion_trusted_revoke,
            companion_snapshot,
            companion_playback_status,
            companion_playback_control,
            companion_library_album,
            companion_library_tracks,
            companion_wifi_status,
            companion_wifi_scan_start,
            companion_wifi_scan_results,
            companion_wifi_connect,
            companion_wifi_connect_slot,
            companion_wifi_disconnect,
            companion_wifi_autoreconnect,
            companion_lastfm_status,
            companion_lastfm_control,
            companion_lastfm_request_token,
            companion_history_summary,
            companion_history_albums,
            companion_history_tracks,
            companion_history_clear,
            companion_bt_status,
            companion_bt_control,
            companion_bt_scan_start,
            companion_bt_scan_results,
            companion_bt_bonded_list,
            companion_bt_unbond,
            companion_output_status,
            companion_output_select,
            companion_wifi_list_slots,
            companion_wifi_save_slot,
            companion_wifi_reconnect,
            companion_hid_status,
            companion_hid_led_set,
            companion_script_status,
            companion_script_list,
            companion_script_log,
            companion_script_run,
            companion_system_reboot,
            companion_system_reboot_download,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
