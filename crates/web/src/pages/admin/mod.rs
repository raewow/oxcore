pub mod accounts;
pub mod audit;
pub mod characters;
pub mod live_map;
pub mod overview;
pub mod permissions;
pub mod shell;

use leptos::prelude::*;

use crate::portal;

/// Shared by the per-account audit section (`accounts::AdminAccountDetail`) and the
/// global audit log page (`audit::AdminAuditLog`).
fn render_audit_entries(result: Result<Vec<portal::AuditEntry>, ServerFnError>) -> AnyView {
    match result {
        Ok(entries) if entries.is_empty() => view! { <p class="mt-3 text-xs text-muted-foreground">"No audit records."</p> }.into_any(),
        Ok(entries) => view! { <ul class="mt-3 divide-y divide-border text-xs">{entries.into_iter().map(|entry| view! { <li class="py-2"><span class="font-medium">{entry.action}</span> " | " {entry.actor.unwrap_or_else(|| "system".to_string())} " | " {entry.occurred_at} {entry.reason.map(|reason| view! { <p class="mt-1 text-muted-foreground">{reason}</p> })}</li> }).collect_view()}</ul> }.into_any(),
        Err(error) => view! { <p class="mt-3 text-xs text-destructive">{error.to_string()}</p> }.into_any(),
    }
}
