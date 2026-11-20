# Property Watch server

Rust gRPC hello world with a dedicated SQLite database. This is the initial server scaffold; property discovery and the other product features are planned separately.

Install Rust using `rust-toolchain.toml` and Protocol Buffers (`protobuf-compiler` on Ubuntu).

```sh
cargo run -- serve
```

In another terminal:

```sh
cargo run -- hello http://127.0.0.1:50061 Scout
```

The reply is `Hello, Scout!`. `cargo run -- check` checks standard gRPC health and makes a database-backed hello request.

| Setting | Local default |
|---|---|
| `PW_BIND_ADDR` | `127.0.0.1:50061` |
| `PW_DATABASE_PATH` | `property-watch.sqlite3` in the current directory |

Keep the port on loopback. Use an SSH tunnel for remote testing; public access requires a TLS gRPC proxy. Database files and credentials are ignored by Git.

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

The integration test exercises real gRPC, health, request-size limits, database failure and persistence after reconnect.
