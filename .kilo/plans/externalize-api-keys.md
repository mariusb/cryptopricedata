# Plan: Externalize API Keys via `.env`

## Goal

Remove all hardcoded API keys from `src/main.rs` and read them at runtime from a `.env` file:
- `GECKO_API_KEY` — used by CoinGecko endpoints (`fetch_api1`, `fetch_api2`, `fetch_api3`)
- `OPENEXCHANGERATES_APP_ID` — used by OpenExchangeRates endpoint (`fetch_api4`)

`fetch_api5` (VALR public market summary) is unauthenticated and stays unchanged.

## Current State

- `src/main.rs:92` — hardcoded `x_cg_demo_api_key=CG-6ECawTrFcx92rJg7UrgtXUjb` in `fetch_api1`
- `src/main.rs:99` — same hardcoded CoinGecko key in `fetch_api2`
- `src/main.rs:106` — same hardcoded CoinGecko key in `fetch_api3`
- `src/main.rs:116` — hardcoded `app_id=3263b0c93523446299d17e2e6abdd748` in `fetch_api4`
- `Cargo.toml` — no `.env`-loading dependency
- `.gitignore` — only ignores `/target` and `*.ods`; does not ignore `.env`
- No `.env` file exists in the repo

## Changes

### 1. `Cargo.toml` — add `dotenvy`

Add to `[dependencies]`:

```toml
dotenvy = "0.15"
```

`dotenvy` is the maintained successor to `dotenv`; it reads a `.env` file in the current working directory and populates `std::env`.

### 2. `.gitignore` — keep secrets out of version control

Append:

```
.env
```

Also add a checked-in template `.env.example` (see step 3) so the file format is discoverable.

### 3. `.env.example` — new file (committed)

Create `.env.example` with placeholder values:

```
# Copy this file to `.env` and fill in real values.
GECKO_API_KEY=your_coingecko_api_key_here
OPENEXCHANGERATES_APP_ID=your_openexchangerates_app_id_here
```

### 4. `src/main.rs` — load env, thread keys into fetch functions

#### 4a. Top of `main()`

At the very start of `main()`, before any other work, load the env file and read the two required keys. Fail fast with a clear error if either is missing.

```rust
dotenvy::dotenv().ok();

let gecko_api_key = std::env::var("GECKO_API_KEY")
    .map_err(|_| "GECKO_API_KEY not set; add it to a .env file or your environment")?;
let openexchangerates_app_id = std::env::var("OPENEXCHANGERATES_APP_ID")
    .map_err(|_| "OPENEXCHANGERATES_APP_ID not set; add it to a .env file or your environment")?;
```

#### 4b. Update fetch function signatures

Pass the keys as `&str` parameters so the functions remain pure (no env lookup at call time, easier to test):

```rust
async fn fetch_api1(api_key: &str) -> Result<PriceMultiResponse, reqwest::Error> { ... }
async fn fetch_api2(api_key: &str) -> Result<PriceMultiResponse, reqwest::Error> { ... }
async fn fetch_api3(api_key: &str) -> Result<PriceResponse, reqwest::Error> { ... }
async fn fetch_api4(app_id: &str) -> Result<ExchangeRatesResponse, reqwest::Error> { ... }
```

`fetch_api5` is unchanged.

#### 4c. URL construction with `format!`

Replace each hardcoded URL with a `format!` call. Example for `fetch_api1`:

```rust
let url = format!(
    "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin,ethereum,dogecoin,tron,cardano,midnight-3,blockdag,tether,usd-coin&vs_currencies=usd&x_cg_demo_api_key={api_key}"
);
let resp = reqwest::get(&url).await?;
```

Apply the same pattern to `fetch_api2`, `fetch_api3` (CoinGecko demo key) and `fetch_api4` (`app_id={app_id}`).

#### 4d. Update `tokio::join!` in `main()`

```rust
let (api1, api2, api3, api4, api5) = tokio::join!(
    fetch_api1(&gecko_api_key),
    fetch_api2(&gecko_api_key),
    fetch_api3(&gecko_api_key),
    fetch_api4(&openexchangerates_app_id),
    fetch_api5(),
);
```

### 5. `README.md` — document the new requirement

In the **Prerequisites** section, add a note that the app reads from a local `.env` file containing `GECKO_API_KEY` and `OPENEXCHANGERATES_APP_ID`. Mention:

- Copy `.env.example` to `.env`
- Replace the placeholders with real keys from CoinGecko and OpenExchangeRates
- `.env` is git-ignored; never commit real keys

## Files Touched

| File | Change |
|---|---|
| `Cargo.toml` | Add `dotenvy = "0.15"` |
| `.gitignore` | Add `.env` |
| `.env.example` | New file with placeholder keys |
| `src/main.rs` | Load env in `main`; thread keys into fetch functions; use `format!` for URLs |
| `README.md` | Document `.env` setup under Prerequisites |

## Verification

After implementation:

1. `cargo build --release` — compile must succeed.
2. Create a local `.env` with test values and run `cargo run --release`. Confirm `CryptoPriceData.ods` gets a new row with non-zero prices (proves the env-injected keys work end-to-end).
3. Unset the variables and run again — program should exit with a clear error naming the missing key.
4. `grep -n "x_cg_demo_api_key=\|app_id=3" src/main.rs` — should return no hits (no leftover hardcoded keys).
5. `git status` — `.env` must not appear (it's git-ignored); `.env.example` should appear as a new tracked file.

## Notes / Decisions

- **Fail fast on missing keys**: chosen over silently defaulting to `""`, which would just produce 0.0 rows from the API and obscure the real problem.
- **Read env once in `main`**: avoids repeated `std::env::var` lookups inside the `tokio::join!` branches and keeps the fetch functions env-agnostic.
- **`.env` git-ignored, `.env.example` committed**: standard practice so collaborators know the required variables without secrets leaking into history.
- **CoinGecko parameter name stays `x_cg_demo_api_key`**: the API still accepts the demo key via that query parameter name for Pro/Demo keys in the `/v3` simple-price endpoint; renaming is out of scope.

## Completion Report

All todos from the implementation session completed successfully:

- [x] Add `dotenvy = "0.15"` to `Cargo.toml` `[dependencies]`.
- [x] Append `.env` to `.gitignore`.
- [x] Create `.env.example` with `GECKO_API_KEY` and `OPENEXCHANGERATES_APP_ID` placeholders.
- [x] Update `src/main.rs`:
  - `fetch_api1`–`fetch_api3` take `api_key: &str` and use `format!` for the CoinGecko URLs.
  - `fetch_api4` takes `app_id: &str` and uses `format!` for the OpenExchangeRates URL.
  - `fetch_api5` left unchanged (unauthenticated VALR endpoint).
  - `main()` calls `dotenvy::dotenv().ok()` at the top, then reads `GECKO_API_KEY` and `OPENEXCHANGERATES_APP_ID` with `.map_err(|_| format!(...).into())?` so the errors convert to `Box<dyn Error>` cleanly.
  - `tokio::join!` now passes `&gecko_api_key` and `&openexchangerates_app_id` to the relevant fetch functions; `fetch_api5()` stays argument-less.
- [x] Update `README.md` Prerequisites section with a new `## API Keys` block documenting the `cp .env.example .env` workflow and the git-ignore behaviour.
- [x] Verify:
  - `cargo build --release` — clean compile.
  - `grep` for the hardcoded key values (`CG-6ECawTrFcx92rJg7UrgtXUjb`, `3263b0c93523446299d17e2e6abdd748`) returns no matches in `src/main.rs`.
  - End-to-end run with the user's real `.env` updated `CryptoPriceData.ods` successfully.
  - Error path verified by hiding `.env` — program exits with code 1 and message `Error: "GECKO_API_KEY not set; add it to a .env file or your environment"`.
  - `git status` shows `.env` is ignored and `.env.example` appears as a new untracked file ready to be committed.
