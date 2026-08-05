//! Tally — a thin receiver for build and test stats.
//!
//! One binary: JSON POSTs in, `SQLite` underneath, a server-rendered
//! trends page out. The mapper thread is the only door to the
//! database.

mod config;
mod format;
mod mapper;
mod model;
mod route;

use axum::Router;
use axum::routing::get;
use tower_http::trace::TraceLayer;
use tracing::Level;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // `tracing` instead of plain logging because context must follow
    // the *task*, not the thread: async tasks hop threads, so
    // thread-local log context lies. Events carry their span's
    // context wherever the task runs.
    //
    // Filtering honors RUST_LOG, the de facto convention (e.g.
    // RUST_LOG=tally=debug,tower_http=debug) — read here rather
    // than through Config because the variable belongs to the
    // tracing ecosystem, not to Tally. `Targets` parses the same
    // target=level syntax as the heavier `EnvFilter` but without
    // dragging in a regex engine: declining a dependency's default
    // features is a decision worth making on purpose — the
    // env-filter default costs four crates for span-field patterns
    // this service will never write.
    let filter = std::env::var("RUST_LOG")
        .ok()
        .and_then(|directives| directives.parse::<Targets>().ok())
        .unwrap_or_else(|| {
            Targets::new()
                .with_target("tally", Level::INFO)
                .with_target("tower_http", Level::INFO)
        });
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .init();
    let config = config::Config::load();
    let mapper = mapper::spawn(&config.db_path);
    // `with_state` hands the sender to the Router; any handler
    // declaring a `State<T>` parameter receives a clone. Unlike a
    // runtime-keyed state map, this is checked at compile time: the
    // Router's type carries the state type, and a handler asking for
    // state the Router doesn't hold is a compile error, not a 500.
    // `TraceLayer` is tower middleware: a Service wrapping every
    // handler. This one line buys a span per request — method, path,
    // status, latency — because the layer speaks the same `Service`
    // contract the handlers do. See it with
    // RUST_LOG=tower_http=debug.
    let app = Router::new()
        .route("/run", get(route::run).post(route::record))
        .route("/run.json", get(route::run))
        .route("/", get(route::index))
        .route("/healthz", get(route::healthz))
        .layer(TraceLayer::new_for_http())
        .with_state(mapper);
    let addr = &config.bind_addr;
    tracing::info!(%addr, "tally listening");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await
}
