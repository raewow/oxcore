use leptos::prelude::*;

use crate::pages::admin::shell::{AdminSection, AdminShell};
use crate::portal;

#[component]
pub fn Admin() -> impl IntoView {
    let overview = Resource::new(|| (), |_| portal::get_admin_overview());
    view! {
        <AdminShell active=AdminSection::Overview>
            <p class="text-xs font-semibold uppercase tracking-[0.3em] text-primary">"Overview"</p>
            <h1 class="mt-4 font-sans text-3xl font-semibold tracking-tight">"GM tools"</h1>
            <p class="mt-3 text-xs text-muted-foreground">"Realm status and player support workload."</p>
            <Suspense fallback=move || view! { <p class="mt-8 text-xs text-muted-foreground">"Loading overview..."</p> }>
                {move || overview.get().map(render_admin_overview)}
            </Suspense>
        </AdminShell>
    }
}

fn render_admin_overview(result: Result<portal::AdminOverview, ServerFnError>) -> AnyView {
    match result {
        Ok(overview) => view! {
            <div class="mt-8 border-y border-border py-5 text-xs">
                <p class="text-muted-foreground">"Open support queue"</p>
                <p class="mt-1 font-sans text-2xl font-semibold text-foreground">{overview.open_support_tickets}</p>
            </div>
            <ul class="mt-6 divide-y divide-border">
                {overview.realms.into_iter().map(|realm| {
                    let state = if realm.online { "Online" } else { "Offline" };
                    view! { <li class="flex items-center justify-between py-3 text-xs"><span class="font-medium text-foreground">{realm.name}</span><span class=if realm.online { "text-primary" } else { "text-muted-foreground" }>{state}</span></li> }
                }).collect_view()}
            </ul>
        }.into_any(),
        Err(error) => view! {
            <p class="mt-8 text-xs text-muted-foreground">"GM tools could not load."</p>
            <pre class="mt-3 overflow-x-auto border border-border bg-input p-3 text-xs text-destructive">{error.to_string()}</pre>
        }.into_any(),
    }
}
