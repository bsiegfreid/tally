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

type Door = web::Data<mpsc::UnboundedSender<Command>>;

/// Window shown by the daily trends table.
const REPORT_DAYS: u32 = 14;
const MAX_KIND_LEN: usize = 64;
const MAX_HOST_LEN: usize = 128;

/// POST /run — accept one run report. Fire-and-forget: 202 means
/// queued for the mapper thread, not yet fsynced.
pub async fn record(door: Door, body: web::Json<NewRun>) -> HttpResponse {
    let run = body.into_inner();
    if run.kind.is_empty()
        || run.kind.len() > MAX_KIND_LEN
        || run.host.is_empty()
        || run.host.len() > MAX_HOST_LEN
    {
        return HttpResponse::BadRequest().body("kind and host are required");
    }
    match door.send(Command::Record(run)) {
        Ok(()) => HttpResponse::Accepted().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}

/// GET /run (and /run.json) — the report, as the negotiated
/// representation.
pub async fn run(req: HttpRequest, door: Door) -> HttpResponse {
    let (reply, rx) = oneshot::channel();
    let cmd = Command::Report {
        days: REPORT_DAYS,
        reply,
    };
    if door.send(cmd).is_err() {
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

/// GET /healthz — liveness. Answering at all is the answer.
pub async fn healthz() -> HttpResponse {
    HttpResponse::NoContent().finish()
}

/// Page shell and stylesheet, authored as plain files in `assets/`
/// and baked into the binary at build time. `{{style}}`, `{{daily}}`
/// and `{{recent}}` are the shell's slots.
const PAGE: &str = include_str!("../assets/page.html");
const STYLE: &str = include_str!("../assets/style.css");

fn esc(s: &str) -> String {
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
            esc(&d.kind),
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
            esc(&r.kind),
            esc(&r.host),
            secs(r.duration_ms),
            esc(&r.detail.to_string()),
        ));
    }
    PAGE.replace("{{style}}", STYLE)
        .replace("{{daily}}", &daily)
        .replace("{{recent}}", &recent)
}
