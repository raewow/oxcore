//! Bnet Server - standalone binary.
//!
//! Serves the Battle.net login flow for 1.14.x clients. Run it alongside the `auth` binary,
//! which keeps serving 1.12 clients on the legacy realmd protocol.

use anyhow::{Context, Result};
use oxcore_bnet::shared::config::{find_config_file, load_toml};
use oxcore_tui::LogSettings;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use tracing::info;

#[derive(Debug, Deserialize)]
struct RootConfig {
    bnet: oxcore_bnet::config::Config,
}

struct Args {
    config_path: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // `gen-certs` is a one-shot tool that needs no config or logging setup; handle it before
    // the server bootstrap so it works on a fresh checkout with no config.toml.
    let mut raw = std::env::args().skip(1);
    if raw.next().as_deref() == Some("gen-certs") {
        return run_gen_certs(raw.collect());
    }

    let args = parse_args();

    let config_path = args
        .config_path
        .map(PathBuf::from)
        .unwrap_or_else(find_config_file);

    let root: RootConfig = load_toml(&config_path).with_context(|| {
        format!(
            "Failed to load configuration from {}",
            config_path.display()
        )
    })?;
    let config = root.bnet;

    oxcore_tui::install_headless_subscriber(&LogSettings {
        console_level: config.log_level,
        file_level: config.log_file_level,
        log_file: Some(resolve_log_file(&config.logs_dir, &config.log_file)),
        wipe: false,
        // Standalone: this process logs one server, which the shared file already covers.
        components: Vec::new(),
    })?;

    info!("bnet server starting up...");

    let (shutdown_tx, _rx) = tokio::sync::broadcast::channel(1);
    let server = oxcore_bnet::serve(config, shutdown_tx.clone()).await?;

    info!(
        portal = %server.config.portal_address(),
        "bnet server ready — patched clients should use this as their portal"
    );

    tokio::signal::ctrl_c().await?;
    info!("shutdown signal received");
    let _ = shutdown_tx.send(());

    Ok(())
}

fn parse_args() -> Args {
    let mut config_path = None;
    let mut args = std::env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-c" | "--config" => config_path = args.next(),
            _ => {}
        }
    }

    Args { config_path }
}

fn resolve_log_file(logs_dir: &Path, log_file: &str) -> PathBuf {
    if logs_dir.as_os_str().is_empty() {
        PathBuf::from(log_file)
    } else {
        logs_dir.join(log_file)
    }
}

/// `bnet gen-certs [--out <dir>] [--host <name>]...`
///
/// Defaults: output to `./certs`, hostnames `localhost` and `127.0.0.1`. Pass `--host` once
/// per name the patched client will resolve — these must match the client's `portal` value
/// plus the patched suffix, and must be hostnames the certificate covers.
fn run_gen_certs(args: Vec<String>) -> Result<()> {
    let mut out_dir = PathBuf::from("./certs");
    let mut hosts: Vec<String> = Vec::new();

    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--out" | "-o" => {
                out_dir = PathBuf::from(it.next().context("--out requires a directory argument")?);
            }
            "--host" | "-h" => {
                hosts.push(it.next().context("--host requires a hostname argument")?);
            }
            other => anyhow::bail!("unknown gen-certs argument: {other}"),
        }
    }

    if hosts.is_empty() {
        hosts = vec!["localhost".to_string(), "127.0.0.1".to_string()];
    }

    oxcore_bnet::gen_certs::run(&out_dir, &hosts)
}
