use leptos::prelude::*;

#[component]
pub fn Button(
    children: Children,
    #[prop(default = "button")] button_type: &'static str,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let class = format!(
        "inline-flex h-8 shrink-0 items-center justify-center gap-1.5 rounded-none border border-transparent bg-primary px-2.5 text-xs font-medium text-primary-foreground transition-colors hover:bg-primary/80 focus-visible:border-ring focus-visible:ring-1 focus-visible:ring-ring/50 disabled:pointer-events-none disabled:opacity-50 {class}"
    );

    view! {
        <button class=class type=button_type>
            {children()}
        </button>
    }
}

#[component]
pub fn Card(children: Children, #[prop(optional)] class: &'static str) -> impl IntoView {
    let class = format!(
        "flex flex-col gap-4 overflow-hidden rounded-none bg-card py-4 text-xs leading-relaxed text-card-foreground ring-1 ring-foreground/10 {class}"
    );

    view! {
        <section class=class>
            {children()}
        </section>
    }
}

#[component]
pub fn Label(
    for_id: &'static str,
    children: Children,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let class = format!("flex items-center gap-2 text-xs leading-none select-none {class}");

    view! {
        <label class=class for=for_id>
            {children()}
        </label>
    }
}

#[component]
pub fn Input(
    id: &'static str,
    name: &'static str,
    #[prop(default = "text")] input_type: &'static str,
    #[prop(optional)] autocomplete: &'static str,
    #[prop(optional)] placeholder: &'static str,
    #[prop(default = true)] required: bool,
) -> impl IntoView {
    let is_password = input_type == "password";
    let (password_visible, set_password_visible) = signal(false);
    let field_type = move || {
        if is_password && password_visible.get() {
            "text"
        } else {
            input_type
        }
    };
    let input_class = if is_password {
        "block h-8 w-full rounded-none border border-input bg-input px-2.5 pr-16 py-1 text-xs text-foreground outline-none transition-colors placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-1 focus-visible:ring-ring/50 disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50"
    } else {
        "block h-8 w-full rounded-none border border-input bg-input px-2.5 py-1 text-xs text-foreground outline-none transition-colors placeholder:text-muted-foreground focus-visible:border-ring focus-visible:ring-1 focus-visible:ring-ring/50 disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50"
    };

    view! {
        <div class="relative mt-2">
            <input
                class=input_class
                id=id
                name=name
                type=field_type
                autocomplete=autocomplete
                placeholder=placeholder
                required=required
            />
            {is_password.then(|| view! {
                <button
                    class="absolute inset-y-0 right-0 px-2.5 text-xs font-medium text-muted-foreground hover:text-primary focus-visible:outline-1 focus-visible:outline-ring"
                    type="button"
                    aria-controls=id
                    aria-label=move || if password_visible.get() { "Hide password" } else { "Show password" }
                    on:click=move |_| set_password_visible.update(|visible| *visible = !*visible)
                >
                    {move || if password_visible.get() { "Hide" } else { "Show" }}
                </button>
            })}
        </div>
    }
}

/// Centered card shell used by the auth pages (login/register/recover) and the
/// signed-in account pages (account/security/activity/support/character).
#[component]
pub fn AuthShell(title: &'static str, subtitle: &'static str, children: Children) -> impl IntoView {
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
pub fn AuthField(
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
