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

use actix_web::{App, HttpServer, web};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config = config::Config::load();
    let mapper = mapper::spawn(&config.db_path);
    println!("tally listening on {}", config.bind_addr);
    let bind = config.bind_addr.clone();
    // The factory closure runs once per worker thread; `move` gives
    // it ownership of `mapper`, and each worker's App gets a clone
    // of the sender (a refcount bump, not a copy — many producers is
    // mpsc's point). `web::Data` is an `Arc` that `app_data` files
    // in the App's type map, keyed by type: a handler declaring a
    // `Data<T>` parameter receives the value registered under that
    // exact `T`. One value per type, so distinct dependencies get
    // distinct types. Gotcha: this wiring is checked at runtime, not
    // compile time — remove this line and handlers still compile,
    // then 500 on every request. The first thing to check when a
    // handler mysteriously 500s is the type map.
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(mapper.clone()))
            .route("/run", web::post().to(route::record))
            .route("/run", web::get().to(route::run))
            .route("/run.json", web::get().to(route::run))
            .route("/", web::get().to(route::index))
            .route("/healthz", web::get().to(route::healthz))
    })
    .bind(bind)?
    .run()
    .await
}
