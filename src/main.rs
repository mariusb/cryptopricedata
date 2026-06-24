#[allow(unused_imports)]
use chrono::{DateTime, Local, NaiveDate, NaiveTime, Utc};
use serde::Deserialize;
use spreadsheet_ods::{CellStyle, CellStyleRef, Sheet, ValueType, WorkBook, read_ods, write_ods};
use spreadsheet_ods::format::{
    create_date_iso_format, create_number_format_fixed,
    FormatNumberStyle, ValueFormatDateTime, ValueFormatTrait,
};
use spreadsheet_ods::style::{StyleOrigin, StyleUse};
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

async fn fetch_api1(api_key: &str) -> Result<PriceMultiResponse, reqwest::Error> {
    let url = format!(
        "https://api.coingecko.com/api/v3/simple/price?ids=bitcoin,ethereum,dogecoin,tron,cardano,midnight-3,blockdag,tether,usd-coin&vs_currencies=usd&x_cg_demo_api_key={api_key}"
    );
    let resp = reqwest::get(&url).await?;
    let data = resp.json::<PriceMultiResponse>().await?;
    Ok(data)
}

async fn fetch_api2(api_key: &str) -> Result<PriceMultiResponse, reqwest::Error> {
    let url = format!(
        "https://api.coingecko.com/api/v3/simple/price?ids=cardano,midnight-3,blockdag,tron,dogecoin,binancecoin,ethereum,tether,usd-coin&vs_currencies=btc&x_cg_demo_api_key={api_key}"
    );
    let resp = reqwest::get(&url).await?;
    let data = resp.json::<PriceMultiResponse>().await?;
    Ok(data)
}

async fn fetch_api3(api_key: &str) -> Result<PriceResponse, reqwest::Error> {
    let url = format!(
        "https://api.coingecko.com/api/v3/exchanges/valr/tickers?coin_ids=bitcoin&x_cg_demo_api_key={api_key}"
    );
    let resp = reqwest::get(&url).await?;
    let data = resp.json::<ValrTickersResponse>().await?;
    let zar = data.tickers.iter()
        .find(|t| t.base.as_deref() == Some("BTC") && t.target.as_deref() == Some("ZAR"))
        .and_then(|t| t.last);
    Ok(PriceResponse { ZAR: zar })
}

async fn fetch_api4(app_id: &str) -> Result<ExchangeRatesResponse, reqwest::Error> {
    let url = format!(
        "https://openexchangerates.org/api/latest.json?app_id={app_id}&symbols=ZAR,THB,KZT"
    );
    let resp = reqwest::get(&url).await?;
    let data = resp.json::<ExchangeRatesResponse>().await?;
    Ok(data)
}

async fn fetch_api5() -> Result<ValrMarketSummary, reqwest::Error> {
    let url = "https://api.valr.com/v1/public/USDTZAR/marketsummary";
    let resp = reqwest::get(url).await?;
    let data = resp.json::<ValrMarketSummary>().await?;
    Ok(data)
}

fn get_timestamp() -> NaiveTime {
    Local::now().time()
}

fn get_datestamp() -> NaiveDate {
    Local::now().date_naive()
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
    while sheet.value(row_idx, 0).value_type() != ValueType::Empty {
        row_idx += 1;
    }
    row_idx
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let gecko_api_key = std::env::var("GECKO_API_KEY")
        .map_err(|_| format!("GECKO_API_KEY not set; add it to a .env file or your environment"))?;
    let openexchangerates_app_id = std::env::var("OPENEXCHANGERATES_APP_ID")
        .map_err(|_| format!("OPENEXCHANGERATES_APP_ID not set; add it to a .env file or your environment"))?;

    let timestamp = get_timestamp();
    let datestamp = get_datestamp();

    let (api1, api2, api3, api4, api5) = tokio::join!(
        fetch_api1(&gecko_api_key),
        fetch_api2(&gecko_api_key),
        fetch_api3(&gecko_api_key),
        fetch_api4(&openexchangerates_app_id),
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

    let mut fmt_8dp = create_number_format_fixed("num_8dp", 8, false);
    fmt_8dp.set_origin(StyleOrigin::Styles);
    fmt_8dp.set_styleuse(StyleUse::Named);
    let fmt_8dp = workbook.add_number_format(fmt_8dp);

    let mut style_8dp = CellStyle::new("style_8dp", &fmt_8dp);
    style_8dp.set_origin(StyleOrigin::Styles);
    style_8dp.set_styleuse(StyleUse::Named);
    style_8dp.set_parent_style(&CellStyleRef::from("Default"));
    let style_8dp = workbook.add_cellstyle(style_8dp);

    let mut fmt_date = create_date_iso_format("date_ymd");
    fmt_date.set_origin(StyleOrigin::Styles);
    fmt_date.set_styleuse(StyleUse::Named);
    let fmt_date = workbook.add_datetime_format(fmt_date);

    let mut style_date = CellStyle::new("style_date", &fmt_date);
    style_date.set_origin(StyleOrigin::Styles);
    style_date.set_styleuse(StyleUse::Named);
    style_date.set_parent_style(&CellStyleRef::from("Default"));
    let style_date = workbook.add_cellstyle(style_date);

    let mut fmt_time = ValueFormatDateTime::new_named("time_hm");
    fmt_time.part_hours().style(FormatNumberStyle::Long).build();
    fmt_time.part_text(":").build();
    fmt_time.part_minutes().style(FormatNumberStyle::Long).build();
    fmt_time.set_origin(StyleOrigin::Styles);
    fmt_time.set_styleuse(StyleUse::Named);
    let fmt_time = workbook.add_datetime_format(fmt_time);

    let mut style_time = CellStyle::new("style_time", &fmt_time);
    style_time.set_origin(StyleOrigin::Styles);
    style_time.set_styleuse(StyleUse::Named);
    style_time.set_parent_style(&CellStyleRef::from("Default"));
    let style_time = workbook.add_cellstyle(style_time);

    let sheet = workbook.sheet_mut(0);
    let row_idx = find_next_empty_row(sheet);

    sheet.set_styled_value(row_idx, 0, datestamp, &style_date);
    sheet.set_styled_value(row_idx, 1, timestamp, &style_time);

    for (col, value) in values.iter().enumerate() {
        sheet.set_styled_value(row_idx, (col + 2) as u32, *value, &style_8dp);
    }

    write_ods(&mut workbook, ODS_FILE)?;

    // println!("Data written to {}", ODS_FILE);
    Ok(())
}
