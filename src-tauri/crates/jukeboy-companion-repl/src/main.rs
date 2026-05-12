#[cfg(target_os = "windows")]
#[tokio::main]
async fn main() {
    if let Err(error) = windows_repl::run().await {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!(
        "jukeboy-companion-repl targets Windows. Build it with --target x86_64-pc-windows-msvc."
    );
}

#[cfg(target_os = "windows")]
mod windows_repl {
    use std::{
        env,
        error::Error,
        io::{self, Write},
        path::PathBuf,
    };

    use jukeboy_companion_core::{
        storage::CredentialStore, AuthRequest, BtControlRequest, CompanionManager, ConnectRequest,
        LastfmControlRequest, PageRequest, PairBeginRequest, PingRequest, PlaybackControlRequest,
        ScanRequest, ToggleRequest, TrustedRevokeRequest, WifiConnectRequest,
        WifiConnectSlotRequest,
    };
    use serde::Serialize;
    use serde_json::Value;

    type ReplResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

    enum CommandResult {
        Continue,
        Exit,
        Json(Value),
    }

    pub async fn run() -> ReplResult<()> {
        let manager = CompanionManager::new(CredentialStore::for_app_data_dir(app_data_dir()));
        let mut event_rx = manager.subscribe_events();

        tokio::spawn(async move {
            loop {
                match event_rx.recv().await {
                    Ok(event) => print_event(&event),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });

        println!("Jukeboy Companion REPL for Windows");
        println!(
            "Credential store: {}",
            manager.credential_store().path().display()
        );
        print_help();

        loop {
            let line = read_prompt_line().await?;
            match run_command(&manager, &line).await {
                Ok(CommandResult::Continue) => {}
                Ok(CommandResult::Exit) => break,
                Ok(CommandResult::Json(value)) => print_json(&value),
                Err(error) => eprintln!("error: {error}"),
            }
        }

        Ok(())
    }

    async fn read_prompt_line() -> io::Result<String> {
        tokio::task::spawn_blocking(|| {
            print!("jukeboy> ");
            io::stdout().flush()?;

            let mut line = String::new();
            let bytes_read = io::stdin().read_line(&mut line)?;
            if bytes_read == 0 {
                Ok("quit".to_string())
            } else {
                Ok(line)
            }
        })
        .await
        .map_err(|error| io::Error::new(io::ErrorKind::Other, error))?
    }

    async fn run_command(manager: &CompanionManager, line: &str) -> ReplResult<CommandResult> {
        let mut parts = line.split_whitespace().collect::<Vec<_>>();
        if parts.is_empty() {
            return Ok(CommandResult::Continue);
        }

        let command = parts.remove(0).to_ascii_lowercase();
        match command.as_str() {
            "help" | "?" => {
                print_help();
                Ok(CommandResult::Continue)
            }
            "quit" | "exit" => Ok(CommandResult::Exit),
            "scan" => to_json(manager.scan(Some(scan_request(&parts))).await?),
            "connect" => to_json(manager.connect(connect_request(&parts)).await?),
            "disconnect" => to_json(manager.disconnect().await?),
            "status" => to_json(manager.status().await?),
            "hello" => to_json(manager.hello().await?),
            "capabilities" | "caps" => to_json(manager.capabilities().await?),
            "ping" => to_json(
                manager
                    .ping(Some(PingRequest {
                        text: Some(parts.join(" ")).filter(|value| !value.is_empty()),
                    }))
                    .await?,
            ),
            "auth" => to_json(manager.auth(Some(auth_request(&parts))).await?),
            "pair" => to_json(manager.pair_begin(pair_request(&parts)).await?),
            "pair-status" => to_json(manager.pair_status().await?),
            "pair-cancel" => to_json(manager.pair_cancel().await?),
            "trusted" => to_json(manager.trusted_list().await?),
            "trusted-revoke" => to_json(
                manager
                    .trusted_revoke(TrustedRevokeRequest {
                        client_id: required_arg(&parts, "client_id")?.to_string(),
                    })
                    .await?,
            ),
            "snapshot" => to_json(manager.snapshot().await?),
            "playback" => to_json(manager.playback_status().await?),
            "next" => playback(manager, "next", None, None, None).await,
            "prev" | "previous" => playback(manager, "previous", None, None, None).await,
            "pause" => playback(manager, "pause_toggle", None, None, None).await,
            "play-index" => {
                playback(manager, "play_index", parse_u32(parts.first())?, None, None).await
            }
            "seek" => {
                playback(
                    manager,
                    "seek_seconds",
                    parse_u32(parts.first())?,
                    None,
                    None,
                )
                .await
            }
            "volume" => {
                playback(
                    manager,
                    "set_volume_percent",
                    parse_u32(parts.first())?,
                    None,
                    None,
                )
                .await
            }
            "mode" => playback(manager, "set_mode", None, parts.first().copied(), None).await,
            "output" => {
                playback(
                    manager,
                    "set_output_target",
                    None,
                    None,
                    parts.first().copied(),
                )
                .await
            }
            "library-album" => to_json(manager.library_album().await?),
            "tracks" => to_json(
                manager
                    .library_track_page(Some(page_request(&parts, 8)))
                    .await?,
            ),
            "wifi" => to_json(manager.wifi_status().await?),
            "wifi-scan" => to_json(manager.wifi_scan_start().await?),
            "wifi-results" => to_json(
                manager
                    .wifi_scan_results(Some(page_request(&parts, 8)))
                    .await?,
            ),
            "wifi-connect" => to_json(
                manager
                    .wifi_connect(WifiConnectRequest {
                        ssid: required_arg(&parts, "ssid")?.to_string(),
                        password: parts.get(1).map(|value| (*value).to_string()),
                    })
                    .await?,
            ),
            "wifi-slot" => to_json(
                manager
                    .wifi_connect_slot(WifiConnectSlotRequest {
                        slot: parse_u32(parts.first())?.unwrap_or(0),
                    })
                    .await?,
            ),
            "wifi-disconnect" => to_json(manager.wifi_disconnect().await?),
            "wifi-auto" => to_json(
                manager
                    .wifi_autoreconnect(ToggleRequest {
                        enabled: parse_bool(required_arg(&parts, "on|off")?)?,
                    })
                    .await?,
            ),
            "lastfm" => to_json(manager.lastfm_status().await?),
            "lastfm-control" => to_json(manager.lastfm_control(lastfm_request(&parts)?).await?),
            "history" => to_json(manager.history_summary().await?),
            "history-albums" => to_json(
                manager
                    .history_album_page(Some(page_request(&parts, 4)))
                    .await?,
            ),
            "bt" => to_json(manager.bt_audio_status().await?),
            "bt-connect-last" => bt(manager, "connect-last").await,
            "bt-pair-best" => bt(manager, "pair-best").await,
            "bt-disconnect" => bt(manager, "disconnect").await,
            unknown => {
                eprintln!("unknown command: {unknown}");
                Ok(CommandResult::Continue)
            }
        }
    }

    async fn playback(
        manager: &CompanionManager,
        action: &str,
        value: Option<u32>,
        mode: Option<&str>,
        output_target: Option<&str>,
    ) -> ReplResult<CommandResult> {
        to_json(
            manager
                .playback_control(PlaybackControlRequest {
                    action: action.to_string(),
                    value,
                    mode: mode.map(ToOwned::to_owned),
                    output_target: output_target.map(ToOwned::to_owned),
                })
                .await?,
        )
    }

    async fn bt(manager: &CompanionManager, action: &str) -> ReplResult<CommandResult> {
        to_json(
            manager
                .bt_audio_control(BtControlRequest {
                    action: action.to_string(),
                })
                .await?,
        )
    }

    fn scan_request(parts: &[&str]) -> ScanRequest {
        ScanRequest {
            scan_timeout_secs: parts.first().and_then(|value| value.parse::<f64>().ok()),
        }
    }

    fn connect_request(parts: &[&str]) -> ConnectRequest {
        let mut request = ConnectRequest {
            address: None,
            name: None,
            profile: None,
            client_id: None,
            app_name: None,
            secret_hex: None,
            timeout_secs: None,
            scan_timeout_secs: None,
        };

        for part in parts {
            if part.eq_ignore_ascii_case("mock") {
                request.profile = Some("mock".to_string());
            } else if let Some((key, value)) = part.split_once('=') {
                match key {
                    "address" => request.address = Some(value.to_string()),
                    "name" => request.name = Some(value.to_string()),
                    "profile" => request.profile = Some(value.to_string()),
                    "client" | "client_id" => request.client_id = Some(value.to_string()),
                    "app" | "app_name" => request.app_name = Some(value.to_string()),
                    "secret" | "secret_hex" => request.secret_hex = Some(value.to_string()),
                    "timeout" => request.timeout_secs = value.parse::<f64>().ok(),
                    "scan" => request.scan_timeout_secs = value.parse::<f64>().ok(),
                    _ => {}
                }
            } else if request.address.is_none() {
                request.address = Some((*part).to_string());
            }
        }

        request
    }

    fn auth_request(parts: &[&str]) -> AuthRequest {
        let mut request = AuthRequest {
            client_id: None,
            app_name: None,
            secret_hex: None,
        };

        for part in parts {
            if let Some((key, value)) = part.split_once('=') {
                match key {
                    "client" | "client_id" => request.client_id = Some(value.to_string()),
                    "app" | "app_name" => request.app_name = Some(value.to_string()),
                    "secret" | "secret_hex" => request.secret_hex = Some(value.to_string()),
                    _ => {}
                }
            }
        }

        request
    }

    fn pair_request(parts: &[&str]) -> PairBeginRequest {
        let mut request = PairBeginRequest {
            client_id: None,
            app_name: Some("jukeboy-companion-repl".to_string()),
            secret_hex: None,
            sequence: None,
            wait: Some(true),
            wait_timeout_secs: Some(120.0),
        };

        for part in parts {
            if *part == "--no-wait" {
                request.wait = Some(false);
            } else if let Some((key, value)) = part.split_once('=') {
                match key {
                    "client" | "client_id" => request.client_id = Some(value.to_string()),
                    "app" | "app_name" => request.app_name = Some(value.to_string()),
                    "secret" | "secret_hex" => request.secret_hex = Some(value.to_string()),
                    "wait" => request.wait = parse_bool(value).ok(),
                    "timeout" => request.wait_timeout_secs = value.parse::<f64>().ok(),
                    "sequence" => request.sequence = Some(parse_sequence(value)),
                    _ => {}
                }
            } else if part.contains(',') {
                request.sequence = Some(parse_sequence(part));
            }
        }

        request
    }

    fn page_request(parts: &[&str], default_count: u32) -> PageRequest {
        PageRequest {
            offset: parts.first().and_then(|value| value.parse::<u32>().ok()),
            count: parts
                .get(1)
                .and_then(|value| value.parse::<u32>().ok())
                .or(Some(default_count)),
        }
    }

    fn lastfm_request(parts: &[&str]) -> ReplResult<LastfmControlRequest> {
        let action = required_arg(parts, "action")?.to_string();
        let mut request = LastfmControlRequest {
            action,
            url: None,
            username: None,
            password: None,
            enabled: None,
        };

        for part in &parts[1..] {
            if let Some((key, value)) = part.split_once('=') {
                match key {
                    "url" => request.url = Some(value.to_string()),
                    "username" | "user" => request.username = Some(value.to_string()),
                    "password" | "pass" => request.password = Some(value.to_string()),
                    "enabled" => request.enabled = Some(parse_bool(value)?),
                    _ => {}
                }
            }
        }

        Ok(request)
    }

    fn parse_sequence(value: &str) -> Vec<String> {
        value
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(ToOwned::to_owned)
            .collect()
    }

    fn parse_u32(value: Option<&&str>) -> ReplResult<Option<u32>> {
        value
            .map(|value| value.parse::<u32>())
            .transpose()
            .map_err(Into::into)
    }

    fn parse_bool(value: &str) -> ReplResult<bool> {
        match value.to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" | "enabled" => Ok(true),
            "0" | "false" | "no" | "off" | "disabled" => Ok(false),
            _ => Err(format!("expected boolean value, got {value}").into()),
        }
    }

    fn required_arg<'a>(parts: &'a [&str], name: &str) -> ReplResult<&'a str> {
        parts
            .first()
            .copied()
            .ok_or_else(|| format!("missing required argument: {name}").into())
    }

    fn to_json<T: Serialize>(value: T) -> ReplResult<CommandResult> {
        Ok(CommandResult::Json(serde_json::to_value(value)?))
    }

    fn print_json(value: &Value) {
        match serde_json::to_string_pretty(value) {
            Ok(encoded) => println!("{encoded}"),
            Err(error) => eprintln!("failed to render response: {error}"),
        }
    }

    fn print_event(event: &Value) {
        match serde_json::to_string(event) {
            Ok(encoded) => println!("\n[event] {encoded}"),
            Err(error) => eprintln!("failed to render event: {error}"),
        }
        print!("jukeboy> ");
        let _ = io::stdout().flush();
    }

    fn print_help() {
        println!("Commands:");
        println!("  scan [secs]                         scan for companion devices");
        println!("  connect [mock|address] [key=value]  connect using BLE or mock backend");
        println!("  disconnect | status                 manage connection state");
        println!("  hello | caps | ping [text]          protocol smoke checks");
        println!("  auth | pair [sequence=a,b,c,d]      authenticate or begin pairing");
        println!("  snapshot | playback | next | pause  inspect and control playback");
        println!("  volume N | seek N | mode NAME       playback value commands");
        println!("  output bluetooth|i2s                choose output target");
        println!("  tracks [offset] [count]             read library tracks");
        println!("  wifi | wifi-scan | wifi-results     inspect Wi-Fi state");
        println!("  bt | bt-connect-last | bt-disconnect inspect Bluetooth audio");
        println!("  help | quit                         show help or exit");
    }

    fn app_data_dir() -> PathBuf {
        env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
            .join("jukeboy-companion-repl")
    }
}
