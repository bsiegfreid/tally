//! The only door to the database. One thread owns the sole `SQLite`
//! connection and drains commands from a channel; handlers never
//! touch the file. Writes are fire-and-forget, reads reply on a
//! oneshot channel.

use rusqlite::{Connection, params};
use tokio::sync::{mpsc, oneshot};

use crate::model::{Daily, NewRun, Report, RunRow};

const SCHEMA: &str = "
    PRAGMA journal_mode = WAL;
    PRAGMA synchronous = NORMAL;
    CREATE TABLE IF NOT EXISTS run (
        id          INTEGER PRIMARY KEY,
        kind        TEXT NOT NULL,
        host        TEXT NOT NULL,
        duration_ms INTEGER NOT NULL,
        detail      TEXT NOT NULL DEFAULT '{}',
        received    INTEGER NOT NULL DEFAULT (unixepoch())
    ) STRICT;
    CREATE INDEX IF NOT EXISTS run_kind_received
        ON run (kind, received);
";

/// Rows in the recent-runs table on the index page.
const RECENT_LIMIT: i64 = 20;

pub enum Command {
    Record(NewRun),
    Report {
        days: u32,
        reply: oneshot::Sender<rusqlite::Result<Report>>,
    },
}

/// Open the database, apply the schema, and hand the connection to a
/// dedicated thread. The returned sender is the door. Panics if the
/// database cannot be opened — there is nothing to serve without it.
pub fn spawn(db_path: &str) -> mpsc::UnboundedSender<Command> {
    // Deliberate panic, and the only two in the binary. `expect` is
    // acceptable here because this runs at startup, before the
    // listener binds: better to die with a clear message than to
    // serve without a database. The rule this example teaches: after
    // the first request is accepted, a failure becomes a response or
    // a log line — never a panic.
    let conn = Connection::open(db_path).expect("open database");
    conn.execute_batch(SCHEMA).expect("apply schema");
    // The thread takes ownership of the Connection (`move`), so no
    // Mutex is ever needed: instead of many threads sharing the
    // handle, one thread owns it and the others send messages. This
    // is Rust's ownership model used as a concurrency design.
    let (tx, mut rx) = mpsc::unbounded_channel();
    std::thread::spawn(move || {
        // A plain OS thread drains the async channel with
        // `blocking_recv`, while handlers `.await` their replies —
        // the bridge between the sync and async worlds.
        while let Some(cmd) = rx.blocking_recv() {
            match cmd {
                Command::Record(run) => {
                    if let Err(e) = record(&conn, &run) {
                        eprintln!("record failed: {e}");
                    }
                }
                Command::Report { days, reply } => {
                    // `let _ =` discards the send error on purpose:
                    // it only fails when the requester has already
                    // dropped its receiver and no longer wants the
                    // answer. Deliberate discards are spelled
                    // `let _ =`, never `.unwrap()`.
                    let _ = reply.send(report(&conn, days));
                }
            }
        }
    });
    tx
}

fn record(conn: &Connection, run: &NewRun) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO run (kind, host, duration_ms, detail)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            run.kind,
            run.host,
            i64::from(run.duration_ms),
            run.detail.to_string(),
        ],
    )?;
    Ok(())
}

fn report(conn: &Connection, days: u32) -> rusqlite::Result<Report> {
    let mut stmt = conn.prepare(
        "SELECT date(received, 'unixepoch') AS day, kind,
                COUNT(*),
                CAST(COALESCE(SUM(json_extract(detail, '$.failed')),
                              0) AS INTEGER),
                CAST(AVG(duration_ms) AS INTEGER)
         FROM run
         WHERE received >= unixepoch() - ?1 * 86400
         GROUP BY day, kind
         ORDER BY day DESC, kind",
    )?;
    // An iterator of `Result` rows collects into a single
    // `Result<Vec<_>>`: the first `Err` stops the loop and becomes
    // the function's return value, so `?` is paid once, not per row.
    let daily = stmt
        .query_map([days], |row| {
            Ok(Daily {
                day: row.get(0)?,
                kind: row.get(1)?,
                runs: row.get(2)?,
                failed: row.get(3)?,
                avg_ms: row.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut stmt = conn.prepare(
        "SELECT datetime(received, 'unixepoch'), kind, host,
                duration_ms, detail
         FROM run ORDER BY id DESC LIMIT ?1",
    )?;
    let recent = stmt
        .query_map([RECENT_LIMIT], |row| {
            // Reads degrade instead of panicking: a corrupt detail
            // blob becomes JSON null, not a dead page.
            let detail: String = row.get(4)?;
            Ok(RunRow {
                received: row.get(0)?,
                kind: row.get(1)?,
                host: row.get(2)?,
                duration_ms: row.get(3)?,
                detail: serde_json::from_str(&detail).unwrap_or(serde_json::Value::Null),
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(Report { daily, recent })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn send_report(tx: &mpsc::UnboundedSender<Command>) -> Report {
        let (reply, rx) = oneshot::channel();
        tx.send(Command::Report { days: 7, reply }).unwrap();
        rx.blocking_recv().unwrap().unwrap()
    }

    #[test]
    fn records_then_reports() {
        let tx = spawn(":memory:");
        let run: NewRun = serde_json::from_str(
            r#"{"kind":"behave","host":"crow","duration_ms":91500,
                "detail":{"scenarios":142,"failed":3}}"#,
        )
        .unwrap();
        tx.send(Command::Record(run)).unwrap();
        let report = send_report(&tx);
        assert_eq!(report.recent.len(), 1);
        assert_eq!(report.recent[0].host, "crow");
        assert_eq!(report.daily.len(), 1);
        let day = &report.daily[0];
        assert_eq!(day.kind, "behave");
        assert_eq!(day.runs, 1);
        assert_eq!(day.failed, 3);
        assert_eq!(day.avg_ms, 91500);
    }

    #[test]
    fn detail_is_optional() {
        let tx = spawn(":memory:");
        let run: NewRun =
            serde_json::from_str(r#"{"kind":"lint","host":"crow","duration_ms":1200}"#).unwrap();
        tx.send(Command::Record(run)).unwrap();
        let report = send_report(&tx);
        assert_eq!(report.daily.len(), 1);
        assert_eq!(report.daily[0].failed, 0);
    }
}
