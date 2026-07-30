use leptos::prelude::*;

use crate::portal;

#[component]
pub fn Home() -> impl IntoView {
    let account = Resource::new(|| (), |_| portal::get_portal_overview());

    view! {
        <main class="min-h-screen bg-background text-foreground">
            <section class="mx-auto flex min-h-screen max-w-5xl flex-col justify-center px-6 py-20">
                <p class="mb-4 text-xs font-semibold uppercase tracking-[0.3em] text-primary">
                    "oxcore"
                </p>
                <h1 class="max-w-3xl font-sans text-5xl font-semibold tracking-tight sm:text-6xl">
                    "The player portal is taking shape."
                </h1>
                <p class="mt-6 max-w-2xl text-lg leading-8 text-muted-foreground">
                    "Account registration, character management, and operations tooling will be available here."
                </p>
                <Suspense fallback=move || view! {
                    <HomeActions authenticated=false />
                }>
                    {move || view! { <HomeActions authenticated=account.get().is_some_and(|result| result.is_ok()) /> }}
                </Suspense>
            </section>
        </main>
    }
}

#[component]
fn HomeActions(authenticated: bool) -> impl IntoView {
    let account_action = if authenticated {
        view! {
            <a class="inline-flex h-8 items-center justify-center rounded-none bg-primary px-3 text-xs font-medium text-primary-foreground hover:bg-primary/80" href="/account">
                "My account"
            </a>
        }
        .into_any()
    } else {
        view! {
            <>
                <a class="inline-flex h-8 items-center justify-center rounded-none bg-primary px-3 text-xs font-medium text-primary-foreground hover:bg-primary/80" href="/register">
                    "Create an account"
                </a>
                <a class="inline-flex h-8 items-center justify-center rounded-none border border-input bg-input px-3 text-xs font-medium text-foreground hover:bg-card" href="/login">
                    "Sign in"
                </a>
            </>
        }
        .into_any()
    };

    view! {
        <div class="mt-10 flex flex-wrap gap-3 text-sm">
            {account_action}
            <a class="inline-flex h-8 items-center justify-center rounded-none border border-input bg-input px-3 text-xs font-medium text-foreground hover:bg-card" href="/status">
                "Realm status"
            </a>
        </div>
    }
}
