# Plan: Add Logging to Crypto Price Tracker (v2)

## Version History

- **v1:** Initial plan — file-based logging via `tracing`, monthly rotation, API/ODS error logging.
- **v2:** Incorporates review feedback (4 changes + 1 minor):
  1. Log missing env vars — promoted from §8.7 recommendation into scope (otherwise the first failure mode is invisible in the log).
  2. Log `API 4 rates: None` — silent zero-fill now emits `warn!` (needs `warn` import).
  3. Rotation no longer overwrites an existing backup — guard before `rename`.
  4. Scope contradiction resolved — `README.md` (`RUST_LOG` docs) is now in scope in §2/§6.
  5. (Minor) Log canonical log path on startup so cron/launchd runs are findable.

## Overview

Add file-based logging to `crypto_price` using the `tracing` ecosystem. No existing application behaviour changes — only log statements are inserted, and `if let Ok` patterns are mechanically rewritten to `match` to expose error values for logging. The two `std::env::var(...).map_err(...)?` closures gain an `error!` call before returning the same `String` error (control flow unchanged).

**Log file:** `cryptoprice.log` (current working directory)
**Rotation:** On the first run of a new month, the previous month's log is renamed to `cryptoprice.log.YYMM` (e.g. `cryptoprice.log.2607` for July 2026). If that backup name already exists, rotation is skipped to avoid overwriting it.

---

## 1. Crate Selection

| Crate | Version | Purpose |
|---|---|---|
| `tracing` | 0.1 | Logging facade — `info!`, `warn!`, `error!` macros |
| `tracing-subscriber` | 0.3 (features: `env-filter`) | Formats log records with timestamps, filters by level |
| `tracing-appender` | 0.2 | Non-blocking file writer, `rolling::never` for no automatic rotation |

**Rationale:** `tracing` is the current Rust best-practice logging framework. It is the successor to `log`, provides structured logging, integrates natively with async/Tokio code, and `tracing-subscriber` gives timestamped output and `RUST_LOG` env-filter support out of the box. `tracing-appender` provides a non-blocking file writer so disk I/O never stalls the async runtime.

**Timestamps:** `tracing-subscriber`'s default `SystemTime` formatter prepends a UTC ISO-8601 timestamp to every log record. No extra configuration needed.

---

## 2. File Changes Summary

| File | Change |
|---|---|
| `Cargo.toml` | Add 3 dependencies |
| `.gitignore` | Add 2 patterns for log files |
| `src/main.rs` | Add 2 imports, 1 constant, 2 functions, ~14 log statements, rewrite 5 `if let Ok` blocks to `match`, add `error!` to 2 existing `map_err` closures |
| `README.md` | Document `RUST_LOG` log-level configuration |

No other files are modified.

---

## 3. Cargo.toml

Add to the `[dependencies]` section:

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
tracing-appender = "0.2"
```

---

## 4. .gitignore

Append:

```
cryptoprice.log
cryptoprice.log.*
```

---

## 5. src/main.rs — Detailed Changes

### 5.1 New Imports

**Line 2** — add `Datelike` to the existing chrono import:

```rust
// BEFORE:
use chrono::{DateTime, Local, NaiveDate, NaiveTime, Utc};

// AFTER:
use chrono::{DateTime, Datelike, Local, NaiveDate, NaiveTime, Utc};
```

**After line 9** (after `use std::path::Path;`), add:

```rust
use tracing::{error, info, warn};
```

(`warn` is required for the API 4 `rates: None` branch in §5.4.3.)

### 5.2 New Constant

**After line 12** (`const ODS_FILE: &str = "CryptoPriceData.ods";`), add:

```rust
const LOG_FILE: &str = "cryptoprice.log";
```

### 5.3 New Functions

**After line 151** (after the `get_datestamp` function, before `create_header_sheet`), add these two functions:

```rust
fn rotate_log_if_needed() {
    let log_path = Path::new(LOG_FILE);
    let metadata = match std::fs::metadata(log_path) {
        Ok(m) => m,
        Err(_) => return, // No log file exists yet, nothing to rotate
    };

    let modified = match metadata.modified() {
        Ok(t) => t,
        Err(_) => return,
    };

    let modified_dt: DateTime<Local> = modified.into();
    let now = Local::now();

    // If the log file was last modified in a different month, back it up
    if (modified_dt.year(), modified_dt.month()) != (now.year(), now.month()) {
        let backup_name = format!("{}.{}", LOG_FILE, modified_dt.format("%y%m"));
        let backup_path = Path::new(&backup_name);
        // v2: never clobber an existing backup — std::fs::rename overwrites on Unix
        if backup_path.exists() {
            eprintln!(
                "Backup {} already exists, skipping rotation to avoid overwrite",
                backup_name
            );
            return;
        }
        if let Err(e) = std::fs::rename(log_path, backup_path) {
            eprintln!("Failed to rotate log file: {}", e);
        }
    }
}

fn init_logging() -> tracing_appender::non_blocking::WorkerGuard {
    rotate_log_if_needed();

    let file_appender = tracing_appender::rolling::never(".", LOG_FILE);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(false)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    guard
}
```

**How rotation works:**
1. `rotate_log_if_needed()` checks if `cryptoprice.log` exists.
2. If it exists, reads its last-modified timestamp.
3. Compares (year, month) of the file vs. now.
4. If they differ → rename `cryptoprice.log` → `cryptoprice.log.YYMM` using the file's own month, **unless that backup already exists** (v2 guard — skip instead of overwriting).
5. Then `tracing_appender::rolling::never` creates a fresh `cryptoprice.log`.
6. Subsequent runs in the same month find a current mtime → no rotation.

**Log format** (each line):
```
2026-08-12T10:30:00.123456Z  INFO Application started
2026-08-12T10:30:01.456789Z ERROR API 1 (CoinGecko USD prices) failed: ...
```

### 5.4 Changes to `main()` Function

#### 5.4.1 Logger Initialization — after `dotenvy::dotenv().ok();` (line 199)

Insert:

```rust
    let _log_guard = init_logging();
    info!("Application started");
    match std::fs::canonicalize(LOG_FILE) {
        Ok(p) => info!("Log file: {}", p.display()),
        Err(_) => info!("Log file: {} (relative to current working directory)", LOG_FILE),
    }
```

The `_log_guard` must live for the entire duration of `main()`. When dropped, it flushes the non-blocking writer. Prefixing with `_` suppresses the unused-variable warning while keeping the binding alive until end of scope.

The canonical-path log (v2 minor) makes the log findable when run from cron/launchd with an unexpected CWD. It never panics — falls back to the relative name.

#### 5.4.2 Env Var Error Logging — replace lines 201–205 (v2: promoted into scope)

`init_logging()` runs before the env-var checks, so `error!` output lands in the log. Control flow is unchanged — the same `String` errors are returned via `?`.

```rust
// BEFORE:
    let gecko_api_key = std::env::var("GECKO_API_KEY")
        .map_err(|_| format!("GECKO_API_KEY not set; add it to a .env file or your environment"))?;
    let openexchangerates_app_id = std::env::var("OPENEXCHANGERATES_APP_ID").map_err(|_| {
        format!("OPENEXCHANGERATES_APP_ID not set; add it to a .env file or your environment")
    })?;

// AFTER:
    let gecko_api_key = std::env::var("GECKO_API_KEY").map_err(|_| {
        error!("GECKO_API_KEY not set; add it to a .env file or your environment");
        format!("GECKO_API_KEY not set; add it to a .env file or your environment")
    })?;
    let openexchangerates_app_id = std::env::var("OPENEXCHANGERATES_APP_ID").map_err(|_| {
        error!("OPENEXCHANGERATES_APP_ID not set; add it to a .env file or your environment");
        format!("OPENEXCHANGERATES_APP_ID not set; add it to a .env file or your environment")
    })?;
```

#### 5.4.3 API Error Logging — rewrite `if let Ok` to `match`

Each of the five API result blocks is rewritten from `if let Ok(data) = apiN { ... } else { ... }` to `match apiN { Ok(data) => { ... }, Err(e) => { error!(...); ... } }`. This exposes the error value for logging without changing any control flow or fallback behaviour.

**API 1** (replace lines 220–234):

```rust
    match api1 {
        Ok(data) => {
            values.push(data.BTC.as_ref().and_then(|e| e.USD).unwrap_or(0.0));
            values.push(data.ETH.as_ref().and_then(|e| e.USD).unwrap_or(0.0));
            values.push(data.DOGE.as_ref().and_then(|e| e.USD).unwrap_or(0.0));
            values.push(data.TRX.as_ref().and_then(|e| e.USD).unwrap_or(0.0));
            values.push(data.ADA.as_ref().and_then(|e| e.USD).unwrap_or(0.0));
            values.push(data.NIGHT.as_ref().and_then(|e| e.USD).unwrap_or(0.0));
            values.push(data.BDAG.as_ref().and_then(|e| e.USD).unwrap_or(0.0));
            values.push(data.USDT.as_ref().and_then(|e| e.USD).unwrap_or(0.0));
            values.push(data.USDC.as_ref().and_then(|e| e.USD).unwrap_or(0.0));
        }
        Err(e) => {
            error!("API 1 (CoinGecko USD prices) failed: {}", e);
            for _ in 0..9 {
                values.push(0.0);
            }
        }
    }
```

**API 2** (replace lines 236–250):

```rust
    match api2 {
        Ok(data) => {
            values.push(data.ADA.as_ref().and_then(|e| e.BTC).unwrap_or(0.0));
            values.push(data.NIGHT.as_ref().and_then(|e| e.BTC).unwrap_or(0.0));
            values.push(data.BDAG.as_ref().and_then(|e| e.BTC).unwrap_or(0.0));
            values.push(data.TRX.as_ref().and_then(|e| e.BTC).unwrap_or(0.0));
            values.push(data.DOGE.as_ref().and_then(|e| e.BTC).unwrap_or(0.0));
            values.push(data.BNB.as_ref().and_then(|e| e.BTC).unwrap_or(0.0));
            values.push(data.ETH.as_ref().and_then(|e| e.BTC).unwrap_or(0.0));
            values.push(data.USDT.as_ref().and_then(|e| e.BTC).unwrap_or(0.0));
            values.push(data.USDC.as_ref().and_then(|e| e.BTC).unwrap_or(0.0));
        }
        Err(e) => {
            error!("API 2 (CoinGecko BTC prices) failed: {}", e);
            for _ in 0..9 {
                values.push(0.0);
            }
        }
    }
```

**API 3** (replace lines 252–256):

```rust
    match api3 {
        Ok(data) => {
            values.push(data.ZAR.unwrap_or(0.0));
        }
        Err(e) => {
            error!("API 3 (CoinGecko VALR BTC/ZAR) failed: {}", e);
            values.push(0.0);
        }
    }
```

**API 4** (replace lines 258–273):

```rust
    match api4 {
        Ok(data) => {
            if let Some(rates) = data.rates {
                values.push(rates.ZAR.unwrap_or(0.0));
                values.push(rates.THB.unwrap_or(0.0));
                values.push(rates.KZT.unwrap_or(0.0));
                values.push(rates.EUR.unwrap_or(0.0));
            } else {
                // v2: previously a silent zero-fill — now warned
                warn!("API 4 (OpenExchangeRates): rates field missing from response");
                for _ in 0..4 {
                    values.push(0.0);
                }
            }
        }
        Err(e) => {
            error!("API 4 (OpenExchangeRates) failed: {}", e);
            for _ in 0..4 {
                values.push(0.0);
            }
        }
    }
```

**API 5** (replace lines 275–284):

```rust
    match api5 {
        Ok(data) => {
            values.push(
                data.last_traded_price
                    .as_ref()
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.0),
            );
        }
        Err(e) => {
            error!("API 5 (VALR USDTZAR market summary) failed: {}", e);
            values.push(0.0);
        }
    }
```

#### 5.4.4 ODS Read Error Logging (best practice addition)

Replace lines 286–291:

```rust
// BEFORE:
    let path = Path::new(ODS_FILE);
    let mut workbook = if path.exists() {
        read_ods(path)?
    } else {
        WorkBook::default()
    };

// AFTER:
    let path = Path::new(ODS_FILE);
    let mut workbook = if path.exists() {
        match read_ods(path) {
            Ok(wb) => wb,
            Err(e) => {
                error!("Failed to read existing ODS file '{}': {}", ODS_FILE, e);
                return Err(e.into());
            }
        }
    } else {
        WorkBook::default()
    };
```

#### 5.4.5 ODS Write Error Logging

Replace line 347:

```rust
// BEFORE:
    write_ods(&mut workbook, ODS_FILE)?;

// AFTER:
    if let Err(e) = write_ods(&mut workbook, ODS_FILE) {
        error!("Failed to write ODS file '{}': {}", ODS_FILE, e);
        return Err(e.into());
    }
```

#### 5.4.6 Finish Log — before `Ok(())` (line 350)

Insert:

```rust
    info!("Application finished successfully");
```

---

## 6. README.md

Document the `RUST_LOG` configuration (resolves v1 scope contradiction where §2 said "no other files" but §7.8 asked for README docs). Append a short section, e.g.:

```markdown
## Logging

Logs are written to `cryptoprice.log` in the current working directory. On the
first run of a new month the previous month's log is rotated to
`cryptoprice.log.YYMM`.

Set `RUST_LOG` to control verbosity (default `info`):

```sh
RUST_LOG=debug cargo run   # verbose output
RUST_LOG=error cargo run   # errors only
```
```

---

## 7. Log Level Summary

| Level | Message | Location |
|---|---|---|
| INFO | `Application started` | Top of `main()`, after logger init |
| INFO | `Log file: {canonical path}` | Top of `main()`, after startup log |
| ERROR | `GECKO_API_KEY not set; ...` | Env-var check (v2, before `?`) |
| ERROR | `OPENEXCHANGERATES_APP_ID not set; ...` | Env-var check (v2, before `?`) |
| ERROR | `API 1 (CoinGecko USD prices) failed: {e}` | API 1 error branch |
| ERROR | `API 2 (CoinGecko BTC prices) failed: {e}` | API 2 error branch |
| ERROR | `API 3 (CoinGecko VALR BTC/ZAR) failed: {e}` | API 3 error branch |
| WARN | `API 4 (OpenExchangeRates): rates field missing from response` | API 4 `rates: None` branch (v2) |
| ERROR | `API 4 (OpenExchangeRates) failed: {e}` | API 4 error branch |
| ERROR | `API 5 (VALR USDTZAR market summary) failed: {e}` | API 5 error branch |
| ERROR | `Failed to read existing ODS file '{file}': {e}` | ODS read error |
| ERROR | `Failed to write ODS file '{file}': {e}` | ODS write error |
| INFO | `Application finished successfully` | End of `main()`, before `Ok(())` |

---

## 8. Recommendations for Additional Logging

The following are **not implemented** in this plan but are recommended for future consideration:

1. **INFO — ODS file creation vs. append.** Log whether a new ODS file was created or an existing one was opened for appending. Helps confirm expected behaviour in production.
   ```rust
   info!("Created new ODS file '{}'", ODS_FILE);  // new file branch
   info!("Opened existing ODS file '{}' for append", ODS_FILE);  // existing file branch
   ```

2. **INFO — Row index written.** Log the row number being written. Useful for debugging data alignment issues.
   ```rust
   info!("Writing data to row {}", row_idx);
   ```

3. **WARN — Missing optional fields in API responses.** When a coin's price field is `None` in the JSON response, log a warning. This catches API schema changes or data gaps that silently produce 0.0 values. (v2 implements the API 4 `rates: None` case in scope; per-coin fields remain future work.)
   ```rust
   if data.BTC.is_none() { warn!("API 1: bitcoin field missing from response"); }
   ```

4. **DEBUG — API call start/end.** Log when each API call begins and completes. Useful for diagnosing latency or timeout issues.
   ```rust
   debug!("Fetching API 1: CoinGecko USD prices");
   ```

5. **DEBUG — API response payload size.** Log the number of bytes or fields received. Helps detect truncated responses.

6. **INFO — Environment variable loading.** Log when API keys are successfully loaded (without printing the key values). Confirms `.env` parsing worked.
   ```rust
   info!("API keys loaded from environment");
   ```

7. ~~**ERROR — Environment variable missing.**~~ **Done in v2** — see §5.4.2.

8. ~~**Configurable log level via `RUST_LOG`.**~~ **Done in v2** — `env-filter` support plus `README.md` docs in §6.

---

## 9. Verification

After implementation, verify:

1. `cargo build` succeeds with no errors.
2. Run the app — `cryptoprice.log` is created in the current directory.
3. Log file contains `INFO Application started` as the first line, followed by the `INFO Log file: ...` path line.
4. Log file contains `INFO Application finished successfully` as the last line (on success).
5. If an API call fails, its `ERROR` line appears in the log with a timestamp.
6. Simulate month boundary: set `cryptoprice.log`'s mtime to a previous month (e.g. `touch -t 202607150000 cryptoprice.log`), run the app — the old file is renamed to `cryptoprice.log.2607` and a new `cryptoprice.log` is created.
7. **(v2)** Rotation guard: pre-create `cryptoprice.log.2607`, set `cryptoprice.log`'s mtime to July 2026, run the app — the existing backup is **not** overwritten (stderr notes the skip) and the app still runs.
8. **(v2)** Unset `GECKO_API_KEY` (and separately `OPENEXCHANGERATES_APP_ID`), run the app — the corresponding `ERROR ... not set` line appears in the log before the process exits non-zero.
9. **(v2)** Simulate an API 4 response with missing `rates` — the `WARN ... rates field missing` line appears.
10. Set `RUST_LOG=debug` and verify no debug output appears (since no debug statements exist yet — confirms filter works).
11. Set `RUST_LOG=error` and verify INFO messages are suppressed.
