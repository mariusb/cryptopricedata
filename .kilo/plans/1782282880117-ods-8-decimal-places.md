# Plan: Display 8 Decimal Places in ODS Value Cells

## Goal
All numeric value cells in the ODS output should display exactly 8 decimal places (e.g., `62714.00000000`, `0.07881100`) instead of the current default of 2.

## Affected File
- `src/main.rs`

---

## What Was Done

### 1. Added imports
```rust
use spreadsheet_ods::{CellStyle, CellStyleRef, Sheet, WorkBook, read_ods, write_ods};
use spreadsheet_ods::format::{create_number_format_fixed, ValueFormatTrait};
use spreadsheet_ods::style::{StyleOrigin, StyleUse};
```

- `CellStyleRef` — needed to reference the parent `"Default"` style.
- `create_number_format_fixed` — helper that creates a `ValueFormatNumber` with fixed decimal places **and** `min-integer-digits="1"`.
- `ValueFormatTrait` — required to call `set_origin` / `set_styleuse` on the format.
- `StyleOrigin`, `StyleUse` — needed to place the style in `styles.xml` as a common named style instead of an automatic style in `content.xml`.

### 2. Created the 8-decimal number format and cell style
```rust
let mut fmt_8dp = create_number_format_fixed("num_8dp", 8, false);
fmt_8dp.set_origin(StyleOrigin::Styles);
fmt_8dp.set_styleuse(StyleUse::Named);
let fmt_8dp = workbook.add_number_format(fmt_8dp);

let mut style_8dp = CellStyle::new("style_8dp", &fmt_8dp);
style_8dp.set_origin(StyleOrigin::Styles);
style_8dp.set_styleuse(StyleUse::Named);
style_8dp.set_parent_style(&CellStyleRef::from("Default"));
let style_8dp = workbook.add_cellstyle(style_8dp);
```

### 3. Changed numeric cell writes to use the styled value
```rust
for (col, value) in values.iter().enumerate() {
    sheet.set_styled_value(row_idx, (col + 2) as u32, *value, &style_8dp);
}
```

Header, datestamp, and timestamp cells continue using `set_value()` — unchanged.

---

## What Went Wrong First

The initial implementation used automatic styles (default `StyleOrigin::Content`, `StyleUse::Automatic`). This placed the format and style in `content.xml` inside `office:automatic-styles`:
```xml
<style:style style:name="style_8dp" style:family="table-cell" style:data-style-name="num_8dp"/>
```

LibreOffice ignored this and displayed values with 2 decimal places, despite the XML being spec-compliant. This is likely a LibreOffice quirk where automatic number styles without proper lineage or language attributes aren't applied to cells.

---

## What Fixed It

Three changes were required:
1. **`StyleOrigin::Styles` + `StyleUse::Named`** on both the format and cell style — this places them in `styles.xml` as a common/named style, matching how LibreOffice natively stores its own custom formats (e.g. `N126` for 8 decimals).
2. **`set_parent_style(&CellStyleRef::from("Default"))`** on the cell style — gives the style a parent lineage so LibreOffice recognizes it as a proper cell style.
3. **`create_number_format_fixed("num_8dp", 8, false)`** instead of manual `part_number().fixed_decimal_places(8).build()` — the helper adds `min-integer-digits="1"`, so values < 1 display a leading zero (e.g. `0.07881100` not `.07881100`).

Resulting XML in `styles.xml`:
```xml
<style:style style:name="style_8dp" style:family="table-cell"
             style:data-style-name="num_8dp" style:parent-style-name="Default"/>
<number:number-style style:name="num_8dp">
  <number:number number:min-integer-digits="1"
                 number:decimal-places="8"
                 number:min-decimal-places="8"/>
</number:number-style>
```

---

## Validation

1. `cargo build` — compiles cleanly.
2. Delete existing `CryptoPriceData.ods`, run `cargo run`, open in LibreOffice — all numeric columns display exactly 8 decimal places with trailing zeros (e.g. `62714.00000000`, `0.07881100`).
3. Header row and date/time columns are unaffected.

## Status: Complete
