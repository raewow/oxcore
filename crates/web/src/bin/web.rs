#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("oxcore_web=info,tower_http=info")
            }),
        )
        .with_target(false)
        .compact()
        .init();

    oxcore_web::run().await
}
