//! CLI for the oxcore client patcher.

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Parser;
use oxcore_patcher::{patch, Patch};

#[derive(Parser, Debug)]
#[command(
    name = "oxcore-patcher",
    about = "Patch a WoW 1.14.x client to connect to an oxcore server"
)]
struct Args {
    /// Path to the client executable (e.g. WowClassic.exe).
    #[arg(short, long)]
    input: PathBuf,

    /// Where to write the patched executable. Defaults to <input> with a .patched suffix.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Domain suffix appended to the `portal` value in WTF/Config.wtf. Must start with a dot
    /// and be no longer than `.actual.battle.net`.
    #[arg(long, default_value = ".localhost")]
    portal: String,

    /// Path to the 256-byte RSA modulus used to sign the certificate bundle, as raw bytes.
    /// Produced by `bnet gen-certs` as signature_modulus.bin.
    #[arg(long)]
    modulus: Option<PathBuf>,

    /// Path to the signed certificate bundle blob to embed. Produced by `bnet gen-certs` as
    /// cert_bundle.bin.
    #[arg(long)]
    cert_bundle: Option<PathBuf>,

    /// Path to the 256-byte RSA modulus the client verifies the modern world server's signatures
    /// with (SMSG_ENTER_ENCRYPTED_MODE / SMSG_CONNECT_TO), as raw bytes. Produced by
    /// `bnet gen-certs` as connect_to_modulus.bin. Required to reach the world server; omit it if
    /// you are only testing the bnet login flow.
    #[arg(long)]
    connect_to_modulus: Option<PathBuf>,

    /// Report what would change without writing anything.
    #[arg(long)]
    dry_run: bool,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_target(false)
        .without_time()
        .init();

    let args = Args::parse();

    let mut data = std::fs::read(&args.input)
        .with_context(|| format!("failed to read {}", args.input.display()))?;

    let mut patches: Vec<Patch> = vec![patch::portal(&data, &args.portal)?];

    match &args.modulus {
        Some(path) => {
            let modulus = std::fs::read(path)
                .with_context(|| format!("failed to read modulus from {}", path.display()))?;
            patches.push(patch::signature_modulus(&data, &modulus)?);
        }
        None => {
            // Without this patch the client still trusts only Blizzard-signed bundles, so it
            // will reject our TLS certificate. Refuse rather than produce a client that fails
            // later with an opaque error.
            bail!(
                "--modulus is required: without replacing the signature modulus the client \
                 cannot trust your server's certificate"
            );
        }
    }

    match &args.cert_bundle {
        Some(path) => {
            let blob = std::fs::read(path)
                .with_context(|| format!("failed to read cert bundle from {}", path.display()))?;
            patches.push(patch::cert_bundle(&data, &blob)?);
        }
        None => {
            // The modulus alone makes the client verify bundles against our key, but the bundle
            // it verifies still lists Blizzard's certificates, not ours. Both patches are
            // needed together.
            bail!(
                "--cert-bundle is required: replacing the modulus without replacing the bundle \
                 leaves the client trusting Blizzard's certificates instead of yours"
            );
        }
    }

    // The connect-to modulus is optional: it is only needed to reach the world server, not for
    // the bnet login flow, so a login-only test can leave it off.
    if let Some(path) = &args.connect_to_modulus {
        let modulus = std::fs::read(path)
            .with_context(|| format!("failed to read connect-to modulus from {}", path.display()))?;
        patches.push(patch::connect_to_modulus(&data, &modulus)?);
    }

    for p in &patches {
        println!("{}", p.describe());
    }

    if args.dry_run {
        println!("\ndry run — nothing written");
        return Ok(());
    }

    let output = args.output.unwrap_or_else(|| {
        let mut path = args.input.clone();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        path.set_file_name(format!("{name}.patched"));
        path
    });

    patch::apply(&mut data, &patches)?;
    std::fs::write(&output, &data)
        .with_context(|| format!("failed to write {}", output.display()))?;

    println!("\nwrote {}", output.display());
    println!("next: set `SET portal \"<your-host>\"` in WTF/Config.wtf, then run the patched exe");

    Ok(())
}
