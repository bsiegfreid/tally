//! HTTP handlers and the server-rendered page. One resource, `/run`,
//! answering as HTML or JSON by content negotiation. No JavaScript:
//! the page refreshes itself with a meta tag and stays fully
//! self-contained.

use actix_web::http::header;
use actix_web::{HttpRequest, HttpResponse, web};
use tokio::sync::{mpsc, oneshot};

use crate::format::Format;
use crate::mapper::Command;
use crate::model::{NewRun, Report};

type Mapper = web::Data<mpsc::UnboundedSender<Command>>;

/// Window shown by the daily trends table.
const REPORT_DAYS: u32 = 14;
const MAX_KIND_LEN: usize = 64;
const MAX_HOST_LEN: usize = 128;

/// POST /run — accept one run report. Fire-and-forget: 202 means
/// queued for the mapper thread, not yet fsynced.
///
/// Handler parameters are actix-web "extractors": declared by type,
/// built by the framework before the body runs. `web::Json<NewRun>`
/// deserializes the request body — malformed JSON is rejected with
/// a 400 before this function is ever called. `Mapper` is the
/// shared state `main` registered with `.app_data()`; actix clones
/// it in per call. Handlers return an `HttpResponse` directly:
/// every outcome, including failure, is a response.
pub async fn record(mapper: Mapper, body: web::Json<NewRun>) -> HttpResponse {
    let run = body.into_inner();
    if run.kind.is_empty()
        || run.kind.len() > MAX_KIND_LEN
        || run.host.is_empty()
        || run.host.len() > MAX_HOST_LEN
    {
        return HttpResponse::BadRequest().body("kind and host are required");
    }
    // After boot, errors are responses, never panics. The send can
    // only fail if the mapper thread is gone; that's a 500, and the
    // process stays up to say so.
    match mapper.send(Command::Record(run)) {
        Ok(()) => HttpResponse::Accepted().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

/// GET /run (and /run.json) — the report, as the negotiated
/// representation.
///
/// Extractor parameters, as on `record`. `HttpRequest` is the
/// whole-request extractor, used here to read the path and `Accept`
/// header for content negotiation.
pub async fn run(req: HttpRequest, mapper: Mapper) -> HttpResponse {
    // `oneshot` is a channel for exactly one value, used exactly
    // once: `reply` is consumed by its `send`, `rx` by its `await`.
    // A fresh pair is made per request and dropped with it, so
    // request/reply correlation needs no ids and no cleanup — the
    // channel itself is the correlation.
    let (reply, rx) = oneshot::channel();
    let cmd = Command::Report {
        days: REPORT_DAYS,
        reply,
    };
    if mapper.send(cmd).is_err() {
        return HttpResponse::InternalServerError().finish();
    }
    let report = match rx.await {
        Ok(Ok(report)) => report,
        _ => return HttpResponse::InternalServerError().finish(),
    };
    let accept = req
        .headers()
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok());
    let format = Format::negotiate(req.path(), accept);
    let body = match format {
        Format::Json => match serde_json::to_string(&report) {
            Ok(json) => json,
            Err(_) => return HttpResponse::InternalServerError().finish(),
        },
        Format::Html => render(&report),
    };
    HttpResponse::Ok()
        .content_type(format.content_type())
        .body(body)
}

/// GET / — the resource lives at /run; send the browser there.
pub async fn index() -> HttpResponse {
    HttpResponse::Found()
        .insert_header((header::LOCATION, "/run"))
        .finish()
}

/// GET /healthz — liveness probe. The path follows the "z-pages"
/// convention from Google's internal services (`/healthz`, `/varz`),
/// spread industry-wide by Kubernetes; the trailing `z` avoids
/// colliding with a real application resource named `health`. A de
/// facto standard, not an RFC. Liveness needs no body and no state:
/// a 204 proves the process is up and serving, which is the whole
/// question. Readiness ("should traffic come here?") is a separate
/// probe this service is simple enough not to need.
pub async fn healthz() -> HttpResponse {
    HttpResponse::NoContent().finish()
}

/// Page shell and stylesheet, authored as plain files in `assets/`
/// and baked into the binary at build time. `{{style}}`, `{{daily}}`
/// and `{{recent}}` are the shell's slots.
const PAGE: &str = include_str!("../assets/page.html");
const STYLE: &str = include_str!("../assets/style.css");

/// Minimal HTML escaping. Every client-supplied string goes through
/// here before it lands in markup — non-negotiable, even on a
/// trusted network.
fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn secs(ms: i64) -> String {
    format!("{:.1}s", ms as f64 / 1000.0)
}

fn render(report: &Report) -> String {
    let mut daily = String::new();
    for d in &report.daily {
        daily.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td class=n>{}</td>\
             <td class=n>{}</td><td class=n>{}</td></tr>\n",
            d.day,
            escape_html(&d.kind),
            d.runs,
            d.failed,
            secs(d.avg_ms),
        ));
    }
    let mut recent = String::new();
    for r in &report.recent {
        recent.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td>\
             <td class=n>{}</td><td>{}</td></tr>\n",
            r.received,
            escape_html(&r.kind),
            escape_html(&r.host),
            secs(r.duration_ms),
            escape_html(&r.detail.to_string()),
        ));
    }
    PAGE.replace("{{style}}", STYLE)
        .replace("{{daily}}", &daily)
        .replace("{{recent}}", &recent)
}
