# oxcore-web

The browser-facing oxcore player portal and administration console. It uses Leptos for
server-side rendering and hydration, Axum for HTTP/WebSocket handling, and Tailwind CSS.

## Development

For a production-equivalent local build, install `cargo-leptos` once:

```bash
cargo install --locked cargo-leptos
rustup target add wasm32-unknown-unknown
```

Then run the full server, WASM, and Tailwind watch pipeline:

```bash
cargo leptos watch -p oxcore-web
```

For a release artifact:

```bash
cargo leptos build --release -p oxcore-web
```

Use `cargo leptos watch -p oxcore-web` for development. It builds the SSR binary, the hydrated
WebAssembly client, and Tailwind assets together. `cargo run --bin web` is useful only for
server-side troubleshooting and does not build browser assets.

`cargo-leptos` manages the standalone Tailwind compiler; Node and a JavaScript package manager
are not required. The server binds according to `[web]` in `config.toml`.

```toml
[web]
bind_ip = "127.0.0.1"
port = 8080
public_base_url = "http://127.0.0.1:8080"
auth_database_url = "mysql://root:root@127.0.0.1:3306/auth"
character_database_url = "mysql://root:root@127.0.0.1:3306/characters"
web_database_url = "postgres://oxcore:oxcore@127.0.0.1:5432/oxcore"
```

Terminate TLS at a reverse proxy in production and bind the web server to a private interface.
