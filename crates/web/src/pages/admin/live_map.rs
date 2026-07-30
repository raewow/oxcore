use leptos::prelude::*;

use crate::pages::admin::shell::{AdminSection, AdminShell};

#[component]
pub fn LiveMap() -> impl IntoView {
    view! {
        <AdminShell active=AdminSection::LiveMap>
            <p class="text-xs font-semibold uppercase tracking-[0.3em] text-primary">"Live map"</p>
            <h1 class="mt-4 font-sans text-3xl font-semibold tracking-tight">"Online players"</h1>
            <p class="mt-3 text-xs text-muted-foreground">"Online characters refresh every 10 seconds. Only Eastern Kingdoms and Kalimdor are shown."</p>
            <div id="live-map" class="mt-6 min-h-[70vh] border border-border bg-[#001d29]" aria-label="Live player map"></div>
        </AdminShell>
        <link rel="stylesheet" href="https://unpkg.com/leaflet@1.9.4/dist/leaflet.css" />
        <script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js" defer></script>
        <script src="/live-map.js" defer></script>
    }
}
