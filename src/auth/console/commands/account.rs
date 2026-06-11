//! Account management console commands.

use crate::auth::context::AuthServer;
use crate::shared::common::AccountType;
use crate::shared::console::command::{CommandContext, CommandInfo};
use anyhow::{anyhow, Result};

const MAX_GM_LEVEL: u8 = AccountType::Console as u8;

pub async fn cmd_account(ctx: &CommandContext<'_, AuthServer>, args: &str) -> Result<String> {
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.is_empty() {
        return Ok(
            "Usage:\n  account create <username> <password> [gmlevel]\n  account setgm <username> [gmlevel]".to_string(),
        );
    }

    match parts[0].to_lowercase().as_str() {
        "create" => cmd_create(ctx, &parts[1..]).await,
        "setgm" => cmd_setgm(ctx, &parts[1..]).await,
        sub => Ok(format!(
            "Unknown account subcommand: {}. Use 'account create' or 'account setgm'.",
            sub
        )),
    }
}

async fn cmd_create(ctx: &CommandContext<'_, AuthServer>, parts: &[&str]) -> Result<String> {
    if parts.len() < 2 {
        return Ok(
            "Usage: account create <username> <password> [gmlevel]".to_string(),
        );
    }

    let username = parts[0];
    let password = parts[1];
    let gmlevel = if parts.len() >= 3 {
        parse_gm_level(parts[2])?
    } else {
        0
    };

    let account_id = ctx
        .context
        .database
        .accounts
        .create_account(username, password)
        .await?;

    if gmlevel > 0 {
        ctx.context
            .database
            .accounts
            .set_gmlevel(account_id, gmlevel)
            .await?;
        ctx.context
            .database
            .accounts
            .upsert_account_access(account_id, gmlevel, -1)
            .await?;
    }

    Ok(format!(
        "Account '{}' created (id: {}, gmlevel: {})",
        username.to_uppercase(),
        account_id,
        gmlevel
    ))
}

async fn cmd_setgm(ctx: &CommandContext<'_, AuthServer>, parts: &[&str]) -> Result<String> {
    if parts.is_empty() {
        return Ok(format!(
            "Usage: account setgm <username> [gmlevel]\n  gmlevel defaults to {} (max)",
            MAX_GM_LEVEL
        ));
    }

    let username = parts[0];
    let gmlevel = if parts.len() >= 2 {
        parse_gm_level(parts[1])?
    } else {
        MAX_GM_LEVEL
    };

    let account = ctx
        .context
        .database
        .accounts
        .find_by_username(&username.to_uppercase())
        .await?
        .ok_or_else(|| anyhow!("Account '{}' not found", username))?;

    ctx.context
        .database
        .accounts
        .set_gmlevel(account.id, gmlevel)
        .await?;
    ctx.context
        .database
        .accounts
        .upsert_account_access(account.id, gmlevel, -1)
        .await?;

    Ok(format!(
        "Set gmlevel for '{}' (id: {}) to {}",
        account.username, account.id, gmlevel
    ))
}

fn parse_gm_level(value: &str) -> Result<u8> {
    let gmlevel: u8 = value
        .parse()
        .map_err(|_| anyhow!("Invalid gmlevel: '{}'", value))?;
    if gmlevel > MAX_GM_LEVEL {
        return Err(anyhow!(
            "gmlevel must be between 0 and {} (got {})",
            MAX_GM_LEVEL,
            gmlevel
        ));
    }
    Ok(gmlevel)
}

pub fn account_info() -> CommandInfo {
    CommandInfo {
        name: "account",
        help: "Manage accounts. Usage: account create|setgm ...",
        min_security: AccountType::Player,
    }
}
