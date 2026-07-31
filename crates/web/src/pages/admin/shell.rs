use leptos::prelude::*;

use crate::components::ui::Card;

/// Every page reachable from the GM tools sidebar. Adding a new admin page means adding
/// a variant here and to [`NAV_SECTIONS`] — the sidebar itself never needs to change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminSection {
    Overview,
    Accounts,
    Characters,
    Chat,
    Moderation,
    LiveMap,
    AuditLogs,
    Permissions,
}

impl AdminSection {
    fn href(self) -> &'static str {
        match self {
            AdminSection::Overview => "/admin",
            AdminSection::Accounts => "/admin/accounts",
            AdminSection::Characters => "/admin/characters",
            AdminSection::Chat => "/admin/chat",
            AdminSection::Moderation => "/admin/moderation",
            AdminSection::LiveMap => "/admin/live-map",
            AdminSection::AuditLogs => "/admin/audit-logs",
            AdminSection::Permissions => "/admin/permissions",
        }
    }

    fn label(self) -> &'static str {
        match self {
            AdminSection::Overview => "Overview",
            AdminSection::Accounts => "Accounts",
            AdminSection::Characters => "Characters",
            AdminSection::Chat => "Chat",
            AdminSection::Moderation => "Moderation",
            AdminSection::LiveMap => "Live Map",
            AdminSection::AuditLogs => "Audit Logs",
            AdminSection::Permissions => "Permissions",
        }
    }
}

const NAV_SECTIONS: &[AdminSection] = &[
    AdminSection::Overview,
    AdminSection::Accounts,
    AdminSection::Characters,
    AdminSection::Chat,
    AdminSection::Moderation,
    AdminSection::LiveMap,
    AdminSection::AuditLogs,
    AdminSection::Permissions,
];

/// Shared layout for every `/admin/*` page: the GM tools sidebar plus a content column.
#[component]
pub fn AdminShell(active: AdminSection, children: Children) -> impl IntoView {
    view! {
        <main class="min-h-screen bg-background p-3 text-foreground sm:p-5">
            <div class="flex min-h-[calc(100vh-1.5rem)] w-full flex-col gap-3 sm:min-h-[calc(100vh-2.5rem)] lg:flex-row">
                <Card class="flex shrink-0 flex-col px-4 py-5 lg:w-56">
                    <a class="text-xs font-semibold uppercase tracking-[0.3em] text-primary" href="/">"oxcore"</a>
                    <p class="mt-4 text-xs text-muted-foreground">"GM tools"</p>
                    <nav class="mt-4 grid gap-1 text-xs" aria-label="GM tools navigation">
                        {NAV_SECTIONS.iter().map(|&section| {
                            let is_active = section == active;
                            let class = if is_active {
                                "border border-primary/40 bg-primary/10 px-3 py-2 font-medium text-primary"
                            } else {
                                "px-3 py-2 text-muted-foreground hover:bg-input hover:text-foreground"
                            };
                            view! {
                                <a class=class href=section.href() aria-current=is_active.then_some("page")>{section.label()}</a>
                            }
                        }).collect_view()}
                    </nav>
                    <a class="mt-8 px-3 py-2 text-xs text-muted-foreground hover:text-primary lg:mt-auto" href="/account">"Return to account"</a>
                </Card>
                <section class="min-w-0 flex-1 px-2 py-5 sm:px-5 lg:px-8">
                    {children()}
                </section>
            </div>
        </main>
    }
}
