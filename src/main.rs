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
    BTC: Option<PriceEntry>,
    #[serde(default)]
    ETH: Option<PriceEntry>,
    #[serde(default)]
    DOGE: Option<PriceEntry>,
    #[serde(default)]
    TRX: Option<PriceEntry>,
    #[serde(default)]
    ADA: Option<PriceEntry>,
    #[serde(default)]
    NIGHT: Option<PriceEntry>,
    #[serde(default)]
    BDAG: Option<PriceEntry>,
    #[serde(default)]
    USDT: Option<PriceEntry>,
    #[serde(default)]
    USDC: Option<PriceEntry>,
    #[serde(default)]
    BNB: Option<PriceEntry>,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize)]
struct PriceEntry {
    USD: Option<f64>,
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

async fn fetch_api1() -> Result<PriceMultiResponse, reqwest::Error> {
    let url = "https://min-api.cryptocompare.com/data/pricemulti?fsyms=BTC,ETH,DOGE,TRX,ADA,NIGHT,BDAG,USDT,USDC&tsyms=USD";
    let resp = reqwest::get(url).await?;
    let data = resp.json::<PriceMultiResponse>().await?;
    Ok(data)
}

async fn fetch_api2() -> Result<PriceMultiResponse, reqwest::Error> {
    let url = "https://min-api.cryptocompare.com/data/pricemulti?fsyms=ADA,NIGHT,BDAG,TRX,DOGE,BNB,ETH,USDT,USDC&tsyms=BTC";
    let resp = reqwest::get(url).await?;
    let data = resp.json::<PriceMultiResponse>().await?;
    Ok(data)
}

async fn fetch_api3() -> Result<PriceResponse, reqwest::Error> {
    let url = "https://min-api.cryptocompare.com/data/price?fsym=BTC&tsyms=ZAR&e=VALR";
    let resp = reqwest::get(url).await?;
    let data = resp.json::<PriceResponse>().await?;
    Ok(data)
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
    // let now: DateTime<Utc> = Utc::now();
    let now: DateTime<Local> = Local::now();
    now.format("%Y-%m-%d %H:%M").to_string()
}

fn create_header_sheet() -> Sheet {
    let mut sheet = Sheet::new("CryptoPriceData");
    let headers: [&str; 24] = [
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

    sheet.set_value(row_idx, 0, timestamp.as_str());

    for (col, value) in values.iter().enumerate() {
        sheet.set_value(row_idx, (col + 1) as u32, *value);
    }

    write_ods(&mut workbook, ODS_FILE)?;

    // println!("Data written to {}", ODS_FILE);
    Ok(())
}
