use leptos::prelude::*;

use crate::pages::admin::shell::{AdminSection, AdminShell};
use crate::portal;

#[component]
pub fn AdminAuditLog() -> impl IntoView {
    let audit = Resource::new(|| (), |_| portal::get_admin_audit_log(None));
    view! {
        <AdminShell active=AdminSection::AuditLogs>
            <p class="text-xs font-semibold uppercase tracking-[0.3em] text-primary">"Audit"</p>
            <h1 class="mt-4 font-sans text-3xl font-semibold tracking-tight">"Global audit log"</h1>
            <p class="mt-3 text-xs text-muted-foreground">"Most recent portal and administration events."</p>
            <Suspense fallback=move || view! { <p class="mt-8 text-xs text-muted-foreground">"Loading audit log..."</p> }>
                {move || audit.get().map(super::render_audit_entries)}
            </Suspense>
        </AdminShell>
    }
}
