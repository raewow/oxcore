//! Command handler registration for world
//!
//! Registers all available chat commands with the command registry.

use super::CommandRegistry;

pub mod debug;
pub mod gm;
pub mod guild;
pub mod help;
pub mod item;
pub mod mail;
pub mod player;
pub mod spell;

/// Register all chat commands with the registry
pub fn register_all_commands(registry: &mut CommandRegistry) {
    // Help commands
    registry.register(help::help_info(), |ctx, args| {
        Box::pin(help::cmd_help(ctx, args))
    });

    // Debug commands
    registry.register(debug::ping_info(), |ctx, args| {
        Box::pin(debug::cmd_ping(ctx, args))
    });
    registry.register(debug::pos_info(), |ctx, args| {
        Box::pin(debug::cmd_pos(ctx, args))
    });
    // Aliases for pos
    registry.register(debug::where_info(), |ctx, args| {
        Box::pin(debug::cmd_pos(ctx, args))
    });
    registry.register(debug::coords_info(), |ctx, args| {
        Box::pin(debug::cmd_pos(ctx, args))
    });

    // Player commands
    registry.register(player::addxp_info(), |ctx, args| {
        Box::pin(player::cmd_addxp(ctx, args))
    });
    registry.register(player::addgold_info(), |ctx, args| {
        Box::pin(player::cmd_addgold(ctx, args))
    });

    // Item commands
    registry.register(item::additem_info(), |ctx, args| {
        Box::pin(item::cmd_additem(ctx, args))
    });

    // Lookup subcommands
    registry.register_subcommand("lookup", item::lookup_item_info(), |ctx, args| {
        Box::pin(item::cmd_lookup_item(ctx, args))
    });
    registry.register_subcommand("lookup", spell::lookup_spell_info(), |ctx, args| {
        Box::pin(spell::cmd_lookup_spell(ctx, args))
    });

    // Guild commands
    registry.register(guild::guild_info(), |ctx, args| {
        Box::pin(guild::cmd_guild(ctx, args))
    });

    // GM commands
    registry.register(gm::kill_info(), |ctx, args| {
        Box::pin(gm::cmd_kill(ctx, args))
    });
    registry.register(gm::mod_info(), |ctx, args| Box::pin(gm::cmd_mod(ctx, args)));
    registry.register(gm::speed_info(), |ctx, args| {
        Box::pin(gm::cmd_speed(ctx, args))
    });

    // Spell commands
    registry.register(spell::cast_info(), |ctx, args| {
        Box::pin(spell::cmd_cast(ctx, args))
    });

    // Mail commands
    registry.register(mail::sendmail_info(), |ctx, args| {
        Box::pin(mail::cmd_sendmail(ctx, args))
    });

    // Teleport commands (+ aliases)
    registry.register(gm::teleport_info(), |ctx, args| {
        Box::pin(gm::cmd_teleport(ctx, args))
    });
    registry.register(gm::tp_info(), |ctx, args| {
        Box::pin(gm::cmd_teleport(ctx, args))
    });
    registry.register(gm::tele_info(), |ctx, args| {
        Box::pin(gm::cmd_teleport(ctx, args))
    });
    registry.register(gm::port_info(), |ctx, args| {
        Box::pin(gm::cmd_teleport(ctx, args))
    });
}
