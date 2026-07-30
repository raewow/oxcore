use leptos::prelude::*;

use crate::components::ui::{AuthField, AuthShell, Button};

#[component]
pub fn Login() -> impl IntoView {
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
pub fn Register() -> impl IntoView {
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
pub fn RecoverAccount() -> impl IntoView {
    view! {
        <AuthShell title="Account recovery" subtitle="Password recovery is not available yet.">
            <p class="mt-8 text-sm leading-6 text-slate-300">
                "Recovery links require outbound email delivery, which has not been configured for this server. Contact a server administrator for account help."
            </p>
            <a class="mt-8 inline-block text-sm text-sky-400 hover:text-sky-300" href="/login">"Return to sign in"</a>
        </AuthShell>
    }
}
