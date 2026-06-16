#[allow(unused_imports)]
use chrono::{DateTime, Local, Utc};
use serde::Deserialize;
use spreadsheet_ods::{Sheet, WorkBook, read_ods, write_ods};
use std::path::Path;

const ODS_FILE: &str = "CryptoPriceData.ods";

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct PriceMultiResponse {
    #[serde(default)]
    #[serde(rename = "bitcoin")]
    BTC: Option<PriceEntry>,
    #[serde(default)]
    #[serde(rename = "ethereum")]
    ETH: Option<PriceEntry>,
    #[serde(default)]
    #[serde(rename = "dogecoin")]
    DOGE: Option<PriceEntry>,
    #[serde(default)]
    #[serde(rename = "tron")]
    TRX: Option<PriceEntry>,
    #[serde(default)]
    #[serde(rename = "cardano")]
    ADA: Option<PriceEntry>,
    #[serde(default)]
    #[serde(rename = "midnight-3")]
    NIGHT: Option<PriceEntry>,
    #[serde(default)]
    #[serde(rename = "blockdag")]
    BDAG: Option<PriceEntry>,
    #[serde(default)]
    #[serde(rename = "tether")]
    USDT: Option<PriceEntry>,
    #[serde(default)]
    #[serde(rename = "usd-coin")]
    USDC: Option<PriceEntry>,
    #[serde(default)]
    #[serde(rename = "binancecoin")]
    BNB: Option<PriceEntry>,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct PriceEntry {
    #[serde(rename = "usd")]
    USD: Option<f64>,
    #[serde(rename = "btc")]
    BTC: Option<f64>,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct PriceResponse {
    ZAR: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ExchangeRatesResponse {
    rates: Option<ExchangeRates>,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct ExchangeRates {
    ZAR: Option<f64>,
    THB: Option<f64>,
    KZT: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct ValrMarketSummary {
    #[serde(rename = "lastTradedPrice")]
    last_traded_price: Option<String>,
}

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

async fn fetch_api1() -> Result<PriceMultiResponse, reqwest::Error> {
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin,ethereum,dogecoin,tron,cardano,midnight-3,blockdag,tether,usd-coin&vs_currencies=usd&x_cg_demo_api_key=CG-6ECawTrFcx92rJg7UrgtXUjb";
    let resp = reqwest::get(url).await?;
    let data = resp.json::<PriceMultiResponse>().await?;
    Ok(data)
}

async fn fetch_api2() -> Result<PriceMultiResponse, reqwest::Error> {
    let url = "https://api.coingecko.com/api/v3/simple/price?ids=cardano,midnight-3,blockdag,tron,dogecoin,binancecoin,ethereum,tether,usd-coin&vs_currencies=btc&x_cg_demo_api_key=CG-6ECawTrFcx92rJg7UrgtXUjb";
    let resp = reqwest::get(url).await?;
    let data = resp.json::<PriceMultiResponse>().await?;
    Ok(data)
}

async fn fetch_api3() -> Result<PriceResponse, reqwest::Error> {
    let url = "https://api.coingecko.com/api/v3/exchanges/valr/tickers?coin_ids=bitcoin&x_cg_demo_api_key=CG-6ECawTrFcx92rJg7UrgtXUjb";
    let resp = reqwest::get(url).await?;
    let data = resp.json::<ValrTickersResponse>().await?;
    let zar = data.tickers.iter()
        .find(|t| t.base.as_deref() == Some("BTC") && t.target.as_deref() == Some("ZAR"))
        .and_then(|t| t.last);
    Ok(PriceResponse { ZAR: zar })
}

async fn fetch_api4() -> Result<ExchangeRatesResponse, reqwest::Error> {
    let url = "https://openexchangerates.org/api/latest.json?app_id=3263b0c93523446299d17e2e6abdd748&symbols=ZAR,THB,KZT";
    let resp = reqwest::get(url).await?;
    let data = resp.json::<ExchangeRatesResponse>().await?;
    Ok(data)
}

async fn fetch_api5() -> Result<ValrMarketSummary, reqwest::Error> {
    let url = "https://api.valr.com/v1/public/USDTZAR/marketsummary";
    let resp = reqwest::get(url).await?;
    let data = resp.json::<ValrMarketSummary>().await?;
    Ok(data)
}

fn get_timestamp() -> String {
    // Return Time for Local
    let now: DateTime<Local> = Local::now();
    now.format("%H:%M").to_string()
}

fn get_datestamp() -> String {
    // Return Date for Local
    let now: DateTime<Local> = Local::now();
    now.format("%Y-%m-%d").to_string()
}

fn create_header_sheet() -> Sheet {
    let mut sheet = Sheet::new("CryptoPriceData");
    let headers: [&str; 25] = [
        "Datestamp",
        "Timestamp",
        "BTC_USD",
        "ETH_USD",
        "DOGE_USD",
        "TRX_USD",
        "ADA_USD",
        "NIGHT_USD",
        "BDAG_USD",
        "USDT_USD",
        "USDC_USD",
        "ADA_BTC",
        "NIGHT_BTC",
        "BDAG_BTC",
        "TRX_BTC",
        "DOGE_BTC",
        "BNB_BTC",
        "ETH_BTC",
        "USDT_BTC",
        "USDC_BTC",
        "BTC_ZAR",
        "ZAR_XR",
        "THB_XR",
        "KZT_XR",
        "USDTZAR_lastTradedPrice",
    ];
    for (col, header) in headers.iter().enumerate() {
        sheet.set_value(0, col as u32, *header);
    }
    sheet
}

fn find_next_empty_row(sheet: &Sheet) -> u32 {
    let mut row_idx: u32 = 0;
    while !sheet.value(row_idx, 0).as_str_or("").is_empty() {
        row_idx += 1;
    }
    row_idx
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let timestamp = get_timestamp();
    let datestamp = get_datestamp();

    let (api1, api2, api3, api4, api5) = tokio::join!(
        fetch_api1(),
        fetch_api2(),
        fetch_api3(),
        fetch_api4(),
        fetch_api5(),
    );

    let mut values: Vec<f64> = Vec::new();

    if let Ok(data) = api1 {
        values.push(data.BTC.as_ref().and_then(|e| e.USD).unwrap_or(0.0));
        values.push(data.ETH.as_ref().and_then(|e| e.USD).unwrap_or(0.0));
        values.push(data.DOGE.as_ref().and_then(|e| e.USD).unwrap_or(0.0));
        values.push(data.TRX.as_ref().and_then(|e| e.USD).unwrap_or(0.0));
        values.push(data.ADA.as_ref().and_then(|e| e.USD).unwrap_or(0.0));
        values.push(data.NIGHT.as_ref().and_then(|e| e.USD).unwrap_or(0.0));
        values.push(data.BDAG.as_ref().and_then(|e| e.USD).unwrap_or(0.0));
        values.push(data.USDT.as_ref().and_then(|e| e.USD).unwrap_or(0.0));
        values.push(data.USDC.as_ref().and_then(|e| e.USD).unwrap_or(0.0));
    } else {
        for _ in 0..9 {
            values.push(0.0);
        }
    }

    if let Ok(data) = api2 {
        values.push(data.ADA.as_ref().and_then(|e| e.BTC).unwrap_or(0.0));
        values.push(data.NIGHT.as_ref().and_then(|e| e.BTC).unwrap_or(0.0));
        values.push(data.BDAG.as_ref().and_then(|e| e.BTC).unwrap_or(0.0));
        values.push(data.TRX.as_ref().and_then(|e| e.BTC).unwrap_or(0.0));
        values.push(data.DOGE.as_ref().and_then(|e| e.BTC).unwrap_or(0.0));
        values.push(data.BNB.as_ref().and_then(|e| e.BTC).unwrap_or(0.0));
        values.push(data.ETH.as_ref().and_then(|e| e.BTC).unwrap_or(0.0));
        values.push(data.USDT.as_ref().and_then(|e| e.BTC).unwrap_or(0.0));
        values.push(data.USDC.as_ref().and_then(|e| e.BTC).unwrap_or(0.0));
    } else {
        for _ in 0..9 {
            values.push(0.0);
        }
    }

    if let Ok(data) = api3 {
        values.push(data.ZAR.unwrap_or(0.0));
    } else {
        values.push(0.0);
    }

    if let Ok(data) = api4 {
        if let Some(rates) = data.rates {
            values.push(rates.ZAR.unwrap_or(0.0));
            values.push(rates.THB.unwrap_or(0.0));
            values.push(rates.KZT.unwrap_or(0.0));
        } else {
            for _ in 0..3 {
                values.push(0.0);
            }
        }
    } else {
        for _ in 0..3 {
            values.push(0.0);
        }
    }

    if let Ok(data) = api5 {
        values.push(
            data.last_traded_price
                .as_ref()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
        );
    } else {
        values.push(0.0);
    }

    let path = Path::new(ODS_FILE);
    let mut workbook = if path.exists() {
        read_ods(path)?
    } else {
        WorkBook::default()
    };

    if workbook.num_sheets() == 0 {
        let sheet = create_header_sheet();
        workbook.push_sheet(sheet);
    }

    let sheet = workbook.sheet_mut(0);
    let row_idx = find_next_empty_row(sheet);

    sheet.set_value(row_idx, 0, datestamp.as_str());
    sheet.set_value(row_idx, 1, timestamp.as_str());

    for (col, value) in values.iter().enumerate() {
        sheet.set_value(row_idx, (col + 2) as u32, *value);
    }

    write_ods(&mut workbook, ODS_FILE)?;

    // println!("Data written to {}", ODS_FILE);
    Ok(())
}
