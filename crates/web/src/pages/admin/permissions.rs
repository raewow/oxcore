use leptos::prelude::*;

use crate::pages::admin::shell::{AdminSection, AdminShell};
use crate::portal;

#[component]
pub fn Permissions() -> impl IntoView {
    view! {
        <AdminShell active=AdminSection::Permissions>
            <p class="text-xs font-semibold uppercase tracking-[0.3em] text-primary">"Permissions"</p>
            <h1 class="mt-4 font-sans text-3xl font-semibold tracking-tight">"Role permissions"</h1>
            <p class="mt-3 text-xs text-muted-foreground">"Static reference for the core security levels and their portal access thresholds."</p>
            <div class="mt-8 overflow-x-auto border border-border">
                <table class="min-w-full text-left text-xs">
                    <thead class="border-b border-border bg-card text-muted-foreground">
                        <tr>
                            <th class="px-4 py-3 font-medium">"Capability"</th>
                            {portal::SECURITY_LEVELS.iter().map(|(level, name)| view! { <th class="px-3 py-3 text-center font-medium">{*level} " " {*name}</th> }).collect_view()}
                        </tr>
                    </thead>
                    <tbody>
                        {portal::PORTAL_CAPABILITIES.iter().map(|(capability, minimum_level)| view! {
                            <tr class="border-b border-border last:border-b-0">
                                <td class="whitespace-nowrap px-4 py-3 font-medium text-foreground">{*capability}</td>
                                {portal::SECURITY_LEVELS.iter().map(move |(level, _)| {
                                    let allowed = *level >= *minimum_level;
                                    view! { <td class=if allowed { "border-l border-border px-3 py-3 text-center text-primary" } else { "border-l border-border px-3 py-3 text-center text-muted-foreground" }>{if allowed { "Allowed" } else { "-" }}</td> }
                                }).collect_view()}
                            </tr>
                        }).collect_view()}
                    </tbody>
                </table>
            </div>
        </AdminShell>
    }
}
