//! Live integration test runner for the Jukeboy companion API.
//!
//! Scans for the device, connects, authenticates (auto-pairs interactively if
//! the profile is not yet trusted), and exercises every opcode the firmware
//! exposes. Run with cargo:
//!
//! ```bash
//! cargo run -p jukeboy-companion-tests -- --profile default
//! ```

use std::{
    io::{self, Write},
    path::PathBuf,
    time::Duration,
};

use clap::Parser;
use jukeboy_companion_core::{
    client::CompanionBleClient,
    error::CompanionError,
    protocol::{
        button_id_to_name, decode_frame, default_scan_timeout, default_timeout,
        generate_pairing_credentials, tlv_string, tlv_u32, tlv_u8, CompanionCredentials, Opcode,
        TlvType,
    },
    storage::CredentialStore,
};
use serde_json::Value;

#[derive(Parser, Debug)]
#[command(about = "Live integration tests for the Jukeboy companion BLE API")]
struct Args {
    /// Credential profile name (also stored in the credential file).
    #[arg(long, default_value = "default")]
    profile: String,

    /// Optional address filter, e.g. "AA:BB:CC:DD:EE:FF".
    #[arg(long)]
    address: Option<String>,

    /// Optional name filter, e.g. "Jukeboy" (uses startswith match in firmware).
    #[arg(long)]
    name: Option<String>,

    /// Skip test cases by name (comma-separated, matches case-insensitively).
    #[arg(long, value_delimiter = ',')]
    skip: Vec<String>,

    /// Custom credential file path. Defaults to ./companion_tests_state.json.
    #[arg(long)]
    credential_path: Option<PathBuf>,

    /// Optional path/name of an existing script to execute during script tests.
    #[arg(long)]
    script_name: Option<String>,

    /// If set, the runner will attempt the reboot tests too (destructive).
    #[arg(long)]
    include_reboot: bool,
}

struct TestResult {
    name: String,
    ok: bool,
    detail: String,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let args = Args::parse();

    let credential_path = args
        .credential_path
        .clone()
        .unwrap_or_else(|| PathBuf::from("companion_tests_state.json"));
    let store = CredentialStore::new(credential_path.clone());

    println!(
        "jukeboy-companion-tests: profile='{}' credentials='{}'",
        args.profile,
        credential_path.display()
    );

    let client = match connect_client(&args).await {
        Ok(value) => value,
        Err(error) => {
            eprintln!("connection failed: {error}");
            std::process::exit(1);
        }
    };

    if let Err(error) = ensure_authenticated(&client, &store, &args.profile).await {
        eprintln!("authentication failed: {error}");
        std::process::exit(2);
    }

    let mut results: Vec<TestResult> = Vec::new();

    let mut run = |name: &str, outcome: Result<Value, CompanionError>| {
        let skipped = args
            .skip
            .iter()
            .any(|item| item.eq_ignore_ascii_case(name));
        if skipped {
            println!("  SKIP {name}");
            results.push(TestResult {
                name: name.to_string(),
                ok: true,
                detail: "skipped".into(),
            });
            return None;
        }
        match outcome {
            Ok(value) => {
                println!("  OK   {name}");
                results.push(TestResult {
                    name: name.to_string(),
                    ok: true,
                    detail: short_detail(&value),
                });
                Some(value)
            }
            Err(error) => {
                println!("  FAIL {name}: {error}");
                results.push(TestResult {
                    name: name.to_string(),
                    ok: false,
                    detail: format!("{error}"),
                });
                None
            }
        }
    };

    println!("\n-- Discovery / handshake --");
    run("hello", client.hello().await);
    run("capabilities", client.capabilities().await);
    run("ping", client.ping("hello-jukeboy").await);
    run("trusted_list", client.trusted_list().await);

    println!("\n-- Snapshot + library --");
    run("snapshot", client.snapshot().await);
    run("playback_status", client.playback_status().await);
    let album_value = run("library_album", client.library_album().await);
    run("library_track_page", client.library_track_page(0, 4).await);

    println!("\n-- WiFi --");
    run("wifi_status", client.wifi_status().await);
    run(
        "wifi_list_slots",
        decode_raw(&client, Opcode::WifiListSlots, vec![]).await,
    );

    println!("\n-- Last.fm --");
    run("lastfm_status", client.lastfm_status().await);
    run(
        "lastfm_request_token",
        decode_raw(&client, Opcode::LastfmRequestToken, vec![]).await,
    );

    println!("\n-- History --");
    run("history_summary", client.history_summary().await);
    run("history_album_page", client.history_album_page(0, 4).await);
    if let Some(checksum) = album_value
        .as_ref()
        .and_then(|value| value["cartridge"]["checksum"].as_u64())
    {
        run(
            "history_track_page",
            decode_raw(
                &client,
                Opcode::HistoryTrackPage,
                vec![
                    tlv_u32(TlvType::CartridgeChecksum as u16, checksum as u32),
                    tlv_u32(TlvType::Offset as u16, 0),
                    tlv_u32(TlvType::Count as u16, 4),
                ],
            )
            .await,
        );
    } else {
        println!("  SKIP history_track_page: no album checksum available");
    }

    println!("\n-- Output --");
    run(
        "output_status",
        decode_raw(&client, Opcode::OutputStatus, vec![]).await,
    );

    println!("\n-- Bluetooth audio --");
    run("bt_audio_status", client.bt_audio_status().await);
    run(
        "bt_bonded_list",
        decode_raw(&client, Opcode::BtBondedList, vec![]).await,
    );
    run(
        "bt_scan_start",
        decode_raw(&client, Opcode::BtScanStart, vec![]).await,
    );
    tokio::time::sleep(Duration::from_secs(8)).await;
    run(
        "bt_scan_results",
        decode_raw(&client, Opcode::BtScanResults, vec![]).await,
    );

    println!("\n-- HID --");
    run(
        "hid_status",
        decode_raw(&client, Opcode::HidStatus, vec![]).await,
    );
    run(
        "hid_led_set color",
        decode_raw(
            &client,
            Opcode::HidLedSet,
            vec![
                tlv_u8(TlvType::HidLedR as u16, 64),
                tlv_u8(TlvType::HidLedG as u16, 32),
                tlv_u8(TlvType::HidLedB as u16, 16),
                tlv_u8(TlvType::HidLedBrightness as u16, 30),
            ],
        )
        .await,
    );
    run(
        "hid_led_set off",
        decode_raw(
            &client,
            Opcode::HidLedSet,
            vec![tlv_u8(TlvType::HidLedOff as u16, 1)],
        )
        .await,
    );

    println!("\n-- Scripts --");
    run(
        "script_status",
        decode_raw(&client, Opcode::ScriptStatus, vec![]).await,
    );
    run(
        "script_list (root)",
        decode_raw(&client, Opcode::ScriptList, vec![]).await,
    );
    if let Some(name) = args.script_name.as_deref() {
        run(
            "script_list (named)",
            decode_raw(
                &client,
                Opcode::ScriptList,
                vec![tlv_string(TlvType::ScriptName as u16, name)],
            )
            .await,
        );
        run(
            "script_run (named)",
            decode_raw(
                &client,
                Opcode::ScriptRun,
                vec![tlv_string(TlvType::ScriptName as u16, name)],
            )
            .await,
        );
        run(
            "script_log (named)",
            decode_raw(
                &client,
                Opcode::ScriptLog,
                vec![
                    tlv_string(TlvType::ScriptName as u16, name),
                    tlv_u32(TlvType::Offset as u16, 0),
                    tlv_u32(TlvType::Count as u16, 512),
                ],
            )
            .await,
        );
    } else {
        println!("  SKIP script_list (named) / script_log: --script-name not provided");
    }

    if args.include_reboot {
        println!("\n-- System (DESTRUCTIVE) --");
        run(
            "system_reboot",
            decode_raw(&client, Opcode::SystemReboot, vec![]).await,
        );
    } else {
        println!("\nSkipping system_reboot — pass --include-reboot to enable.");
    }

    let total = results.len();
    let failed = results.iter().filter(|r| !r.ok).count();
    println!("\n=== summary: {}/{} passed ===", total - failed, total);
    for result in &results {
        let tag = if result.ok { "OK  " } else { "FAIL" };
        println!("  {tag} {} :: {}", result.name, truncate(&result.detail, 120));
    }
    if failed > 0 {
        std::process::exit(3);
    }
}

fn short_detail(value: &Value) -> String {
    let raw = value.to_string();
    truncate(&raw, 160)
}

fn truncate(input: &str, limit: usize) -> String {
    if input.len() <= limit {
        input.to_string()
    } else {
        format!("{}…", &input[..limit])
    }
}

async fn connect_client(args: &Args) -> Result<CompanionBleClient, CompanionError> {
    println!("scanning for device…");
    let scan_timeout = default_scan_timeout();
    let request_timeout = default_timeout();
    CompanionBleClient::connect(
        args.address.as_deref(),
        args.name.as_deref(),
        &args.profile,
        scan_timeout,
        request_timeout,
    )
    .await
}

async fn ensure_authenticated(
    client: &CompanionBleClient,
    store: &CredentialStore,
    profile: &str,
) -> Result<(), CompanionError> {
    let hello = client.hello().await?;
    let already_auth = hello["authenticated"].as_bool().unwrap_or(false);
    if already_auth {
        println!("hello: already authenticated");
        return Ok(());
    }

    if let Some(creds) = store.get_credentials(profile)? {
        match auth_with_credentials(client, &creds).await {
            Ok(true) => {
                println!("auth: existing credentials accepted");
                return Ok(());
            }
            Ok(false) => println!("auth: existing credentials rejected — initiating pairing"),
            Err(error) => println!("auth attempt failed: {error} — initiating pairing"),
        }
    } else {
        println!("auth: no stored credentials for profile '{profile}' — initiating pairing");
    }

    interactive_pair(client, store, profile).await
}

async fn auth_with_credentials(
    client: &CompanionBleClient,
    creds: &CompanionCredentials,
) -> Result<bool, CompanionError> {
    let challenge = client.auth_challenge(&creds.client_id).await?;
    let nonce_hex = challenge["nonce_hex"]
        .as_str()
        .ok_or(CompanionError::InvalidNonce)?;
    let nonce = hex::decode(nonce_hex).map_err(|_| CompanionError::InvalidNonce)?;
    let secret = creds.secret()?;
    let result = client.auth_proof(&creds.client_id, &secret, &nonce).await?;
    Ok(result["authenticated"].as_bool().unwrap_or(false))
}

async fn interactive_pair(
    client: &CompanionBleClient,
    store: &CredentialStore,
    profile: &str,
) -> Result<(), CompanionError> {
    let creds = generate_pairing_credentials(
        None,
        Some("jukeboy-companion-tests".to_string()),
        None,
    );
    // Random 4-button sequence
    let sequence: Vec<u8> = (0..4).map(|i| ((i * 7 + 3) % 6) as u8).collect();
    let pretty_sequence: Vec<&'static str> =
        sequence.iter().copied().map(button_id_to_name).collect();
    println!(
        "Pairing required. Press these buttons on the device when prompted: {:?}",
        pretty_sequence
    );
    let secret = creds.secret()?;
    client
        .pair_begin(&creds.client_id, &creds.app_name, &secret, &sequence)
        .await?;

    // Poll until paired or user cancels
    for attempt in 0..60u32 {
        tokio::time::sleep(Duration::from_secs(2)).await;
        let status = client.pair_status().await?;
        let pending = status["pairing_pending"].as_bool().unwrap_or(false);
        let progress = status["pairing_progress"].as_u64().unwrap_or(0);
        let required = status["pairing_required"].as_u64().unwrap_or(4);
        if !pending && progress == 0 && attempt > 0 {
            // pairing completed (or canceled). Try auth.
            break;
        }
        println!("  pairing progress: {}/{}", progress, required);
        if !pending && progress == required && required > 0 {
            break;
        }
    }

    let challenge = client.auth_challenge(&creds.client_id).await?;
    let nonce_hex = challenge["nonce_hex"]
        .as_str()
        .ok_or(CompanionError::InvalidNonce)?;
    let nonce = hex::decode(nonce_hex).map_err(|_| CompanionError::InvalidNonce)?;
    let proof = client.auth_proof(&creds.client_id, &secret, &nonce).await?;
    let authenticated = proof["authenticated"].as_bool().unwrap_or(false);
    if !authenticated {
        return Err(CompanionError::Protocol(
            "pairing completed but auth still fails".into(),
        ));
    }

    store.put_credentials(profile, creds.clone())?;
    println!("pairing succeeded; credentials saved to profile '{profile}'");
    Ok(())
}

async fn decode_raw(
    client: &CompanionBleClient,
    opcode: Opcode,
    tlvs: Vec<Vec<u8>>,
) -> Result<Value, CompanionError> {
    let payload: Vec<u8> = tlvs.into_iter().flatten().collect();
    let frame = client.raw_request(opcode as u16, payload).await?;
    decode_frame(&frame)
}

#[allow(dead_code)]
fn read_user_line(prompt: &str) -> io::Result<String> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut line = String::new();
    io::stdin().read_line(&mut line)?;
    Ok(line.trim().to_string())
}
