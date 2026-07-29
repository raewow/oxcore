use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{components::Route, components::Router, components::Routes, StaticSegment};

/// Document shell used by Axum when server-side rendering a route.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone() />
                <HydrationScripts options/>
                <MetaTags/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
}

/// Root Leptos application shared by server-side rendering and browser hydration.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet id="oxcore-web" href="/pkg/oxcore-web.css" />
        <Title text="oxcore" />
        <Router>
            <Routes fallback=|| view! { <NotFound /> }>
                <Route path=StaticSegment("") view=Home />
            </Routes>
        </Router>
    }
}

#[component]
fn Home() -> impl IntoView {
    view! {
        <main class="min-h-screen bg-slate-950 text-slate-100">
            <section class="mx-auto flex min-h-screen max-w-5xl flex-col justify-center px-6 py-20">
                <p class="mb-4 text-sm font-semibold uppercase tracking-[0.3em] text-sky-400">
                    "oxcore"
                </p>
                <h1 class="max-w-3xl text-5xl font-semibold tracking-tight sm:text-6xl">
                    "The player portal is taking shape."
                </h1>
                <p class="mt-6 max-w-2xl text-lg leading-8 text-slate-300">
                    "Account registration, character management, and operations tooling will be available here."
                </p>
                <div class="mt-10 flex flex-wrap gap-3 text-sm text-slate-400">
                    <span class="rounded-full border border-slate-700 px-4 py-2">"Player portal"</span>
                    <span class="rounded-full border border-slate-700 px-4 py-2">"Admin console"</span>
                    <span class="rounded-full border border-slate-700 px-4 py-2">"Live operations"</span>
                </div>
            </section>
        </main>
    }
}

#[component]
fn NotFound() -> impl IntoView {
    view! {
        <main class="grid min-h-screen place-items-center bg-slate-950 px-6 text-slate-100">
            <section class="max-w-md text-center">
                <p class="text-sm font-semibold uppercase tracking-[0.3em] text-sky-400">"404"</p>
                <h1 class="mt-4 text-3xl font-semibold">"Page not found"</h1>
                <a class="mt-6 inline-block text-sky-400 hover:text-sky-300" href="/">"Return home"</a>
            </section>
        </main>
    }
}
