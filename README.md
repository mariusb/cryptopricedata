# Crypto Price Tracker

A Rust application that fetches cryptocurrency and forex prices from multiple APIs and outputs them to a LibreOffice Calc (.ods) spreadsheet.

## Prerequisites

- Rust (install from https://rust-lang.org)

## API Keys

This app reads API keys from a local `.env` file in the project root. Copy the example file and fill in real values:

```bash
cp .env.example .env
```

Then edit `.env` and replace the placeholders with real keys:

- `GECKO_API_KEY` — from [CoinGecko](https://www.coingecko.com/en/api)
- `OPENEXCHANGERATES_APP_ID` — from [OpenExchangeRates](https://openexchangerates.org/)

The `.env` file is git-ignored; never commit real keys. The committed `.env.example` documents the required schema.

## Build and Run

```bash
cargo build --release
./target/release/crypto_price
```

Or run directly with:
```bash
cargo run --release
```

## Output File

**File:** `CryptoPriceData.ods` (created in the current working directory)

### Columns

| Column | Source | Description |
|--------|--------|-------------|
| Timestamp | Local | Date/time in YYYY-MM-DD HH:MM format |
| BTC_USD | API 1 | Bitcoin price in USD |
| ETH_USD | API 1 | Ethereum price in USD |
| DOGE_USD | API 1 | Dogecoin price in USD |
| TRX_USD | API 1 | TRON price in USD |
| ADA_USD | API 1 | Cardano price in USD |
| NIGHT_USD | API 1 | Night price in USD |
| BDAG_USD | API 1 | BlockDAG price in USD |
| USDT_USD | API 1 | Tether price in USD |
| USDC_USD | API 1 | USD Coin price in USD |
| ADA_BTC | API 2 | Cardano price in BTC |
| NIGHT_BTC | API 2 | Night price in BTC |
| BDAG_BTC | API 2 | BlockDAG price in BTC |
| TRX_BTC | API 2 | TRON price in BTC |
| DOGE_BTC | API 2 | Dogecoin price in BTC |
| BNB_BTC | API 2 | BNB price in BTC |
| ETH_BTC | API 2 | Ethereum price in BTC |
| USDT_BTC | API 2 | Tether price in BTC |
| USDC_BTC | API 2 | USD Coin price in BTC |
| BTC_ZAR | API 3 | Bitcoin price in ZAR (VALR) |
| ZAR_XR | API 4 | South African Rand exchange rate (USD base) |
| THB_XR | API 4 | Thai Baht exchange rate (USD base) |
| KZT_XR | API 4 | Kazakhstani Tenge exchange rate (USD base) |
| USDTZAR_lastTradedPrice | API 5 | Last traded USDT/ZAR price (VALR) |

## API Sources

1. **CoinGecko Simple Price (USD):** `https://api.coingecko.com/api/v3/simple/price`
2. **CoinGecko Simple Price (BTC):** `https://api.coingecko.com/api/v3/simple/price`
3. **CoinGecko VALR Exchange Tickers (BTC/ZAR):** `https://api.coingecko.com/api/v3/exchanges/valr/tickers`
4. **OpenExchangeRates:** `https://openexchangerates.org/api/latest.json`
5. **VALR Market Summary:** `https://api.valr.com/v1/public/USDTZAR/marketsummary`

## Behavior

- If `CryptoPriceData.ods` does not exist, a new file is created with headers
- If the file exists, a new row is appended with fresh data
- Failed API calls result in `0.0` values being written