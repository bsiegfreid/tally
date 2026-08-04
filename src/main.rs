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
    let door = mapper::spawn(&config.db_path);
    println!("tally listening on {}", config.bind_addr);
    let bind = config.bind_addr.clone();
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(door.clone()))
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
