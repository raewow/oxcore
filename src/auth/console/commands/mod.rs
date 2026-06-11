//! Console command implementations for the auth server.

pub mod account;
pub mod server;

use crate::auth::context::AuthServer;
use crate::shared::console::CommandRegistry;

pub fn register_all_commands(registry: &mut CommandRegistry<AuthServer>) {
    registry.register(server::help_info(), |ctx, args| {
        Box::pin(server::cmd_help(ctx, args))
    });
    registry.register(server::info_info(), |ctx, args| {
        Box::pin(server::cmd_info(ctx, args))
    });
    registry.register(server::shutdown_info(), |ctx, args| {
        Box::pin(server::cmd_shutdown(ctx, args))
    });

    registry.register(account::account_info(), |ctx, args| {
        Box::pin(account::cmd_account(ctx, args))
    });
}
