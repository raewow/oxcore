//! Console Command Implementations for world
//!
//! Contains all command handler implementations for world server.

pub mod debug;
pub mod server;

use crate::shared::console::CommandRegistry;
use crate::world::World;

/// Register all commands in the registry
pub fn register_all_commands(registry: &mut CommandRegistry<World>) {
    // Server management commands
    registry.register(server::help_info(), |ctx, args| {
        Box::pin(server::cmd_help(ctx, args))
    });
    registry.register(server::info_info(), |ctx, args| {
        Box::pin(server::cmd_info(ctx, args))
    });
    registry.register(server::shutdown_info(), |ctx, args| {
        Box::pin(server::cmd_shutdown(ctx, args))
    });

    // Debug commands
    registry.register(debug::stats_info(), |ctx, args| {
        Box::pin(debug::cmd_stats(ctx, args))
    });
}
