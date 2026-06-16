# Plan: Switch APIs 1–3 to CoinGecko

## Goal
Replace the three CryptoCompare API calls in `src/main.rs` with CoinGecko equivalents while keeping the ODS file layout and the values written to each column byte-for-byte equivalent. APIs 4 and 5 are unchanged.

## Affected file
- `src/main.rs` (only file that needs edits)

## Background — what the new APIs return

**API 1 (USD prices)** — response keys are CoinGecko coin IDs (lowercase, hyphenated), e.g.:
```json
{ "bitcoin": {"usd": 12345.6}, "ethereum": {"usd": 4321.0}, "midnight-3": {"usd": 0.5}, ... }
```

**API 2 (BTC prices)** — same idea, `vs_currencies=btc`:
```json
{ "cardano": {"btc": 0.000012}, "midnight-3": {"btc": 0.0000003}, ... }
```

**API 3 (BTC/ZAR on VALR)** — `/exchanges/valr/tickers?coin_ids=bitcoin` returns:
```json
{ "name": "VALR", "tickers": [ { "base": "BTC", "target": "ZAR", "last": 1234567.89, ... }, ... ] }
```
The BTC/ZAR price we need is `last` from the ticker entry where `base == "BTC"` and `target == "ZAR"`.

## Changes in `src/main.rs`

### 1. Update `PriceEntry` (lines 34–39) — rename serde fields to lowercase
Add `#[serde(rename = "usd")]` on `USD` and `#[serde(rename = "btc")]` on `BTC` so the same struct works for both API 1 (`usd`) and API 2 (`btc`) responses. Keep the field names (and the rest of the code that reads `entry.USD` / `entry.BTC`) unchanged.

```rust
struct PriceEntry {
    #[serde(rename = "usd")]
    USD: Option<f64>,
    #[serde(rename = "btc")]
    BTC: Option<f64>,
}
```

### 2. Update `PriceMultiResponse` (lines 9–32) — rename fields to CoinGecko IDs
Add a `#[serde(rename = "<coingecko-id>")]` to every field so the existing code that reads `data.BTC`, `data.ETH`, `data.DOGE`, `data.TRX`, `data.ADA`, `data.NIGHT`, `data.BDAG`, `data.USDT`, `data.USDC`, `data.BNB` keeps working unchanged. Mapping:

| Field | `#[serde(rename = ...)]` |
|---|---|
| `BTC` | `"bitcoin"` |
| `ETH` | `"ethereum"` |
| `DOGE` | `"dogecoin"` |
| `TRX` | `"tron"` |
| `ADA` | `"cardano"` |
| `NIGHT` | `"midnight-3"` |
| `BDAG` | `"blockdag"` |
| `USDT` | `"tether"` |
| `USDC` | `"usd-coin"` |
| `BNB` | `"binancecoin"` |

No other changes to this struct (the `Option`/`#[serde(default)]` attributes already in place remain correct for the new responses).

### 3. Update `fetch_api1` URL (line 67)
Replace the URL string with the new CoinGecko URL from the task. Function body, return type, and call site stay the same.

### 4. Update `fetch_api2` URL (line 74)
Replace the URL string with the new CoinGecko URL from the task. Function body, return type, and call site stay the same.

### 5. Add response structs for the VALR tickers endpoint
Add two new structs (placed near the existing `ValrMarketSummary` struct, ~line 60):

```rust
#[derive(Debug, Deserialize)]
struct ValrTickersResponse {
    tickers: Vec<ValrTicker>,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct ValrTicker {
    base: Option<String>,
    target: Option<String>,
    last: Option<f64>,
}
```

All fields are `Option` so the many other fields in CoinGecko's ticker objects are ignored without causing a deserialization error.

### 6. Rewrite `fetch_api3` (lines 80–85) to use the new endpoint
Keep the return type `Result<PriceResponse, reqwest::Error>` and keep the existing `PriceResponse { ZAR: Option<f64> }` struct unchanged so the downstream `data.ZAR.unwrap_or(0.0)` call (line 204) keeps working. Implementation: deserialize into `ValrTickersResponse`, scan for the first ticker with `base == Some("BTC")` and `target == Some("ZAR")`, take its `last`, and wrap it in `PriceResponse { ZAR: Some(price) }`. If no such ticker is found, return `PriceResponse { ZAR: None }` (downstream `unwrap_or(0.0)` will produce 0.0, matching the current behavior on missing data).

```rust
async fn fetch_api3() -> Result<PriceResponse, reqwest::Error> {
    let url = "https://api.coingecko.com/api/v3/exchanges/valr/tickers?coin_ids=bitcoin&x_cg_demo_api_key=CG-6ECawTrFcx92rJg7UrgtXUjb";
    let resp = reqwest::get(url).await?;
    let data = resp.json::<ValrTickersResponse>().await?;
    let zar = data.tickers.iter()
        .find(|t| t.base.as_deref() == Some("BTC") && t.target.as_deref() == Some("ZAR"))
        .and_then(|t| t.last);
    Ok(PriceResponse { ZAR: zar })
}
```

## What stays the same (do NOT touch)
- `ODS_FILE` constant
- `create_header_sheet` and the 25-column header order
- `find_next_empty_row`
- `fetch_api4` and `fetch_api5` (OpenExchangeRates and VALR public)
- The `tokio::join!` block and the order/number of `values.push(...)` calls in `main`
- Column → value mapping in `main` (cols 2..26 are filled from `values[0..23]` in the same order as today)
- `Cargo.toml` (no new dependencies; `reqwest`, `serde`, `serde_json` are already present)

## Verification
After implementing:
1. `cargo build` — should compile without warnings related to the changed code.
2. `cargo run` — should write a new row to `CryptoPriceData.ods` with the same column layout. The 23 numeric columns should be populated from the new CoinGecko responses in the same order as before.
3. Open `CryptoPriceData.ods` and confirm the header row is unchanged and the new data row has 25 cells in the expected order.

## Completion

- [x] Add `ValrTickersResponse` and `ValrTicker` structs near `ValrMarketSummary`
- [x] Update `PriceEntry` with `#[serde(rename = "usd")]` and `#[serde(rename = "btc")]`
- [x] Add `#[serde(rename = "...")]` attributes to all fields in `PriceMultiResponse`
- [x] Update `fetch_api1` URL to CoinGecko `/simple/price` (USD)
- [x] Update `fetch_api2` URL to CoinGecko `/simple/price` (BTC)
- [x] Rewrite `fetch_api3` to use CoinGecko `/exchanges/valr/tickers` endpoint
- [x] `cargo build` passes successfully
