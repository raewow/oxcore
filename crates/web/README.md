# oxcore-web

The browser-facing oxcore player portal and administration console. It uses Leptos for
server-side rendering and hydration, Axum for HTTP/WebSocket handling, and Tailwind CSS.

## Development

Install `cargo-leptos` once:

```bash
cargo install --locked cargo-leptos
```

Then run the full server, WASM, and Tailwind watch pipeline:

```bash
cargo leptos watch -p oxcore-web
```

For a release artifact:

```bash
cargo leptos build --release -p oxcore-web
```

To run the server without the Leptos asset pipeline:

```bash
cargo run --bin web
```

`cargo-leptos` manages the standalone Tailwind compiler; Node and a JavaScript package manager
are not required. The server binds according to `[web]` in `config.toml`.

```toml
[web]
bind_ip = "127.0.0.1"
port = 8080
public_base_url = "http://127.0.0.1:8080"
```

Terminate TLS at a reverse proxy in production and bind the web server to a private interface.
