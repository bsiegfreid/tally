# Tally

A thin receiver for build and test stats, and a worked example of how
small a useful service can stay. JSON POSTs in, SQLite underneath, a
server-rendered trends page out. One binary, no reverse proxy, no
JavaScript, five short modules — the whole shape holds in one head.

Tally serves without TLS or authentication by design; run it inside a
trusted network boundary, or put a proxy in front if it must face one.

## API

One resource, `/run`. `POST /run` reports a run; `GET /run` is the
report, and content negotiation picks the representation — the
`.json` path extension is canonical (`GET /run.json`), `Accept:
application/json` is a courtesy fallback, HTML is the default. `/`
redirects to `/run`.

`POST /run` takes a JSON body. `kind` and `host` are required;
`detail` is free-form JSON for kind-specific numbers, so a new stat
kind needs no schema or code change.

```json
{
  "kind": "behave",
  "host": "runner01",
  "duration_ms": 91500,
  "detail": { "scenarios": 142, "failed": 3 }
}
```

Returns `202 Accepted` (queued for the writer thread). The report
carries daily aggregates for 14 days and the 20 most recent runs; the
HTML page refreshes itself once a minute. `GET /healthz` is liveness.

If a `detail` carries a numeric `failed`, the daily table sums it.

## Configuration

| Variable     | Default        | Purpose             |
| ------------ | -------------- | ------------------- |
| `TALLY_ADDR` | `0.0.0.0:8080` | bind address        |
| `TALLY_DB`   | `tally.sqlite` | SQLite file path    |

## Reporting from behave

`features/environment.py` on the runner:

```python
import json, socket, time, urllib.request

TALLY = "http://tally.internal:8080/run"

def before_all(context):
    context.tally = {"t0": time.monotonic(),
                     "scenarios": 0, "failed": 0}

def after_scenario(context, scenario):
    context.tally["scenarios"] += 1
    if str(scenario.status).endswith("failed"):
        context.tally["failed"] += 1

def after_all(context):
    t = context.tally
    body = json.dumps({
        "kind": "behave",
        "host": socket.gethostname(),
        "duration_ms": int((time.monotonic() - t["t0"]) * 1000),
        "detail": {"scenarios": t["scenarios"],
                   "failed": t["failed"]},
    }).encode()
    req = urllib.request.Request(
        TALLY, data=body,
        headers={"Content-Type": "application/json"})
    try:
        urllib.request.urlopen(req, timeout=5)
    except OSError:
        pass  # stats must never fail the build
```

## Build and run

```sh
cargo run                       # local, tally.sqlite in the cwd
cargo test                      # mapper tests, in-memory database
```

Container (Apple `container` locally; the commands are
Docker-compatible, so `docker` works the same on the server):

```sh
container build -t tally .
container run -d --name tally -p 8080:8080 \
  -v tally-data:/data tally
```

## Shape

Five modules, singular names: `config` (all env reads), `model`
(plain types), `format` (content negotiation, pure), `mapper` (the
only door to SQLite — one thread owns the sole connection), `route`
(handlers and row rendering). The page shell and stylesheet are
plain files in `assets/`, baked into the binary at build time with
`include_str!` — one artifact ships either way.

## License

MIT — see [LICENSE](LICENSE).
