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

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let config = config::Config::load();
    let mapper = mapper::spawn(&config.db_path);
    // `with_state` hands the sender to the Router; any handler
    // declaring a `State<T>` parameter receives a clone. Unlike a
    // runtime-keyed state map, this is checked at compile time: the
    // Router's type carries the state type, and a handler asking for
    // state the Router doesn't hold is a compile error, not a 500.
    let app = Router::new()
        .route("/run", get(route::run).post(route::record))
        .route("/run.json", get(route::run))
        .route("/", get(route::index))
        .route("/healthz", get(route::healthz))
        .with_state(mapper);
    println!("tally listening on {}", config.bind_addr);
    let listener = tokio::net::TcpListener::bind(&config.bind_addr).await?;
    axum::serve(listener, app).await
}
