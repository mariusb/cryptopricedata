# Plan: ODS Date/Time Cells — Native Types Instead of Text

## Goal
Write the first column (Datestamp) as a native ODS **date** cell displaying `YYYY-MM-DD`, and the second column (Timestamp) as a native ODS **time** cell displaying `HH:MM`. Currently both are stored as plain text strings.

## Affected File
- `src/main.rs`

## Context: spreadsheet-ods Value & Format Model
Investigated `spreadsheet-ods` v1.0.4 (pinned in `Cargo.lock`):

- `Value` enum: `Empty | Boolean | Number | Percentage | Currency | Text | TextXml | DateTime(NaiveDateTime) | TimeDuration(Duration)`.
- `From<NaiveDate> for Value` — converts to `Value::DateTime(date + 00:00:00)`.
- `From<NaiveTime> for Value` — converts to `Value::DateTime(1899-12-30 + time)` (LibreOffice's epoch convention).
- `create_date_iso_format(name)` — returns `ValueFormatDateTime` with `YYYY-MM-DD` layout.
- `ValueFormatDateTime::new_named(name)` — builder API with `part_hours()`, `part_minutes()`, etc., lets us build a custom `HH:MM` time-of-day format.

Because `NaiveDate`/`NaiveTime` both map to `Value::DateTime`, the format must be a `ValueFormatDateTime` (not `ValueFormatTimeDuration`, which requires `Duration`).

## Implementation Steps

### 1. Add imports in `src/main.rs`
```rust
use chrono::{DateTime, Local, NaiveDate, NaiveTime, Utc};
use spreadsheet_ods::{CellStyle, CellStyleRef, Sheet, Value, ValueType, WorkBook, read_ods, write_ods};
use spreadsheet_ods::format::{
    create_date_iso_format, create_number_format_fixed,
    ValueFormatDateTime, ValueFormatTrait,
};
use spreadsheet_ods::style::{StyleOrigin, StyleUse};
```
- Add `NaiveDate`, `NaiveTime` from `chrono`.
- Add `Value`, `ValueType` from `spreadsheet_ods`.
- Add `create_date_iso_format`, `ValueFormatDateTime` from `spreadsheet_ods::format`.

### 2. Change `get_timestamp` / `get_datestamp` to return chrono types
```rust
fn get_timestamp() -> NaiveTime {
    Local::now().time()
}

fn get_datestamp() -> NaiveDate {
    Local::now().date_naive()
}
```

### 3. Register a date format + style and a time format + style
After the workbook is opened/created (near the existing `fmt_8dp`/`style_8dp` block), add:

```rust
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
```

Also add `FormatNumberStyle` to the format import.

### 4. Use styled writes for columns 0 and 1
Replace:
```rust
sheet.set_value(row_idx, 0, datestamp.as_str());
sheet.set_value(row_idx, 1, timestamp.as_str());
```
With:
```rust
sheet.set_styled_value(row_idx, 0, datestamp, &style_date);
sheet.set_styled_value(row_idx, 1, timestamp, &style_time);
```
`NaiveDate` and `NaiveTime` both implement `Into<Value>`, so `set_styled_value` accepts them.

### 5. Fix `find_next_empty_row` (critical)
Current implementation:
```rust
while !sheet.value(row_idx, 0).as_str_or("").is_empty() { row_idx += 1; }
```
`as_str_or("")` returns `""` for any non-`Text` value, including `DateTime`. After the change, **every data row's column-0 would look empty**, and the loop would forever stop at the first data row, overwriting it each run.

Fix — check `ValueType::Empty`:
```rust
fn find_next_empty_row(sheet: &Sheet) -> u32 {
    let mut row_idx: u32 = 0;
    while sheet.value(row_idx, 0).value_type() != ValueType::Empty {
        row_idx += 1;
    }
    row_idx
}
```
This works for both the header row (`Text("Datestamp")`) and new DateTime data rows, and stops at the first genuinely empty row.

### 6. Remove now-unused helper string formatting
The `get_datestamp`/`get_timestamp` functions no longer need to format strings; the chrono types flow straight through.

## Risks & Verification

### Risks
- **LibreOffice format support for custom `ValueFormatDateTime`**: The `create_datetime_format` helper (which uses the same part_hours/part_minutes API) is used elsewhere; the builder is well-established. The `time_hm` format is the same, with fewer parts.
- **Style placement**: Using `StyleOrigin::Styles` + `StyleUse::Named` (same pattern as the existing `style_8dp` which works correctly in LibreOffice).
- **Existing rows in `CryptoPriceData.ods`**: Old text rows remain text; new rows become DateTime. This is acceptable (mixed types in a column) and doesn't affect `find_next_empty_row` because `ValueType::Empty` still correctly skips all populated rows.

### Validation
1. `cargo build` — compiles cleanly (no warnings about unused imports/variables).
2. Delete existing `CryptoPriceData.ods`, run `cargo run` twice.
3. Open in LibreOffice:
   - Column A cells format as Date (`YYYY-MM-DD`), not text (Format → Cells shows Date category).
   - Column B cells format as Time (`HH:MM`), not text (Format → Cells shows Time category).
   - Second run appends a **new** row (does not overwrite row 1) — proves `find_next_empty_row` fix works.
   - Numeric columns (C onward) still display 8 decimal places — unchanged.

## Open Questions
None — design decisions resolved above.

---

## Implementation Complete (2026-06-24)

### Changes Made

**1. Imports Updated** (`src/main.rs:1-8`)
- Added `NaiveDate`, `NaiveTime` from `chrono`
- Added `ValueType` from `spreadsheet_ods`
- Added `create_date_iso_format`, `FormatNumberStyle`, `ValueFormatDateTime` from `spreadsheet_ods::format`

**2. Function Return Types Changed** (`src/main.rs:139-145`)
```rust
fn get_timestamp() -> NaiveTime {
    Local::now().time()
}

fn get_datestamp() -> NaiveDate {
    Local::now().date_naive()
}
```

**3. Date Format + Style Registered** (`src/main.rs:303-312`)
```rust
let mut fmt_date = create_date_iso_format("date_ymd");
fmt_date.set_origin(StyleOrigin::Styles);
fmt_date.set_styleuse(StyleUse::Named);
let fmt_date = workbook.add_datetime_format(fmt_date);

let mut style_date = CellStyle::new("style_date", &fmt_date);
style_date.set_origin(StyleOrigin::Styles);
style_date.set_styleuse(StyleUse::Named);
style_date.set_parent_style(&CellStyleRef::from("Default"));
let style_date = workbook.add_cellstyle(style_date);
```

**4. Time Format + Style Registered** (`src/main.rs:314-328`)
```rust
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
```

**5. Styled Writes for Date/Time Columns** (`src/main.rs:332-333`)
```rust
sheet.set_styled_value(row_idx, 0, datestamp, &style_date);
sheet.set_styled_value(row_idx, 1, timestamp, &style_time);
```

**6. Critical Bug Fix in `find_next_empty_row`** (`src/main.rs:186-191`)
```rust
fn find_next_empty_row(sheet: &Sheet) -> u32 {
    let mut row_idx: u32 = 0;
    while sheet.value(row_idx, 0).value_type() != ValueType::Empty {
        row_idx += 1;
    }
    row_idx
}
```
Replaced `.as_str_or("").is_empty()` with `.value_type() != ValueType::Empty` because `as_str_or("")` returns `""` for any non-Text value (including DateTime), which would cause the function to treat all data rows as empty and overwrite them.

### Completed Tasks
- [x] Update imports (NaiveDate, NaiveTime, ValueType, create_date_iso_format, ValueFormatDateTime, FormatNumberStyle)
- [x] Change get_timestamp/get_datestamp to return NaiveTime/NaiveDate
- [x] Register date format+style and time format+style in main()
- [x] Replace set_value with set_styled_value for columns 0 and 1
- [x] Fix find_next_empty_row to use ValueType::Empty check
- [x] Build and verify compilation — clean build with no warnings

### Code Review Results

**Review Date:** 2026-06-24  
**Recommendation:** **APPROVED**

**Security Track:** `NO_FINDINGS`  
No security issues. All inputs come from `Local::now()` (system clock), no network/input handling changes.

**Business Logic Track:** 1 suggestion (non-blocking)

| Severity | File:Line | Issue |
|----------|-----------|-------|
| SUGGESTION | src/main.rs:303-327 | Date/time formats and styles re-registered every run, including when appending to an existing workbook — accumulates duplicate named entries over time |

**Details:** `workbook.add_datetime_format()` and `workbook.add_cellstyle()` are called unconditionally every run. When the ODS file already exists, the loaded workbook already contains the previously-registered `date_ymd`, `time_hm`, `style_date`, and `style_time` definitions; re-registering appends duplicate named entries to the workbook's internal vector, causing unbounded growth. This also applies to the pre-existing `style_8dp` registration.

**Suggested Future Fix:** Guard registration with a check for whether the workbook is newly created vs loaded, or look up existing styles by name.

**Core Logic Validation:**
- `ValueType::Empty` is correct for `find_next_empty_row` (header rows are `Text`, data rows are `DateTime`, both are non-empty)
- Native type flow through `NaiveDate`/`NaiveTime` → `Value::DateTime` is well-understood
- Style registration follows the established codebase pattern exactly
- Changes are backwards-compatible with existing ODS files (old text rows remain text; new rows become DateTime)
