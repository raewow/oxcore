mod components;

use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::{components::Route, components::Router, components::Routes, StaticSegment};

use crate::components::ui::{Button, Card, Input, Label};

const DEV_TAILWIND_THEME: &str = r#"
@theme inline {
  --font-sans: "Cinzel", ui-serif, serif;
  --color-background: oklch(0.12 0.01 250);
  --color-foreground: oklch(0.95 0.01 90);
  --color-card: oklch(0.15 0.01 250);
  --color-card-foreground: oklch(0.95 0.01 90);
  --color-primary: oklch(0.75 0.15 75);
  --color-primary-foreground: oklch(0.12 0.01 250);
  --color-muted-foreground: oklch(0.65 0.02 90);
  --color-border: oklch(0.25 0.02 250);
  --color-input: oklch(0.2 0.01 250);
  --color-ring: oklch(0.75 0.15 75);
}

@layer base {
  body {
    background: oklch(0.12 0.01 250);
    color: oklch(0.95 0.01 90);
    font-family: Inter, ui-sans-serif, system-ui, sans-serif;
  }
}
"#;

/// Document shell used by Axum when server-side rendering a route.
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <meta name="theme-color" content="#1a1a2e"/>
                <meta name="description" content="oxcore World of Warcraft server portal and administration console"/>
                <link rel="preconnect" href="https://fonts.googleapis.com"/>
                <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin="anonymous"/>
                <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Cinzel:wght@400;500;600;700&family=Geist+Mono:wght@400;500&family=Inter:wght@400;500;600;700&display=swap"/>
                {cfg!(debug_assertions).then(|| view! {
                    <style id="tailwind-dev-theme">{DEV_TAILWIND_THEME}</style>
                    <script>{"document.getElementById('tailwind-dev-theme').setAttribute('type', 'text/tailwindcss');"}</script>
                    <script src="https://cdn.jsdelivr.net/npm/@tailwindcss/browser@4"></script>
                })}
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
                <Route path=StaticSegment("login") view=Login />
                <Route path=StaticSegment("register") view=Register />
                <Route path=StaticSegment("recover") view=RecoverAccount />
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
                <div class="mt-10 flex flex-wrap gap-3 text-sm">
                    <a class="rounded-full bg-sky-400 px-4 py-2 font-semibold text-slate-950 hover:bg-sky-300" href="/register">
                        "Create an account"
                    </a>
                    <a class="rounded-full border border-slate-700 px-4 py-2 text-slate-300 hover:border-slate-500" href="/login">
                        "Sign in"
                    </a>
                </div>
            </section>
        </main>
    }
}

#[component]
fn Login() -> impl IntoView {
    view! {
        <AuthShell title="Sign in" subtitle="Use the same account credentials you use in game.">
            <form class="mt-8 space-y-5" action="/auth/login" method="post">
                <AuthField id="login-username" name="username" label="Account name" input_type="text" autocomplete="username" />
                <AuthField id="login-password" name="password" label="Password" input_type="password" autocomplete="current-password" />
                <Button button_type="submit" class="w-full">"Sign in"</Button>
            </form>
            <div class="mt-6 flex justify-between text-sm">
                <a class="text-sky-400 hover:text-sky-300" href="/recover">"Forgot password?"</a>
                <a class="text-slate-400 hover:text-slate-200" href="/register">"Create account"</a>
            </div>
        </AuthShell>
    }
}

#[component]
fn Register() -> impl IntoView {
    view! {
        <AuthShell title="Create your account" subtitle="One password works for both supported game clients and this portal.">
            <form class="mt-8 space-y-5" action="/auth/register" method="post">
                <AuthField id="register-username" name="username" label="Account name" input_type="text" autocomplete="username" />
                <AuthField id="register-email" name="email" label="Email address" input_type="email" autocomplete="email" />
                <AuthField id="register-password" name="password" label="Password" input_type="password" autocomplete="new-password" />
                <Button button_type="submit" class="w-full">"Create account"</Button>
            </form>
            <p class="mt-6 text-sm leading-6 text-slate-400">
                "Email verification will be required once outbound email delivery is configured."
            </p>
            <p class="mt-4 text-sm text-slate-400">
                "Already have an account? "
                <a class="text-sky-400 hover:text-sky-300" href="/login">"Sign in"</a>
            </p>
        </AuthShell>
    }
}

#[component]
fn RecoverAccount() -> impl IntoView {
    view! {
        <AuthShell title="Account recovery" subtitle="Password recovery is not available yet.">
            <p class="mt-8 text-sm leading-6 text-slate-300">
                "Recovery links require outbound email delivery, which has not been configured for this server. Contact a server administrator for account help."
            </p>
            <a class="mt-8 inline-block text-sm text-sky-400 hover:text-sky-300" href="/login">"Return to sign in"</a>
        </AuthShell>
    }
}

#[component]
fn AuthShell(title: &'static str, subtitle: &'static str, children: Children) -> impl IntoView {
    view! {
        <main class="grid min-h-screen place-items-center bg-background px-6 py-12 text-foreground">
            <Card class="w-full max-w-md px-4 py-8 sm:px-6">
                <a class="text-xs font-semibold uppercase tracking-[0.3em] text-primary" href="/">"oxcore"</a>
                <h1 class="mt-6 font-sans text-3xl font-semibold tracking-tight">{title}</h1>
                <p class="mt-3 text-xs leading-6 text-muted-foreground">{subtitle}</p>
                {children()}
            </Card>
        </main>
    }
}

#[component]
fn AuthField(
    id: &'static str,
    name: &'static str,
    label: &'static str,
    input_type: &'static str,
    autocomplete: &'static str,
) -> impl IntoView {
    view! {
        <Label for_id=id>{label}</Label>
        <Input id=id name=name input_type=input_type autocomplete=autocomplete />
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
