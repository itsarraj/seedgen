use fake::faker::address::en::{CityName, CountryName};
use fake::faker::internet::en::FreeEmail;
use fake::faker::lorem::en::{Sentence, Word};
use fake::faker::name::en::Name;
use fake::faker::phone_number::en::PhoneNumber;
use fake::Fake;
use rand::rngs::StdRng;
use rand::Rng;
use serde_json::{json, Map, Value};

use crate::schema::{Column, ColumnType, Schema};

fn random_uuid_v4(rng: &mut StdRng) -> String {
    let mut bytes = [0u8; 16];
    rng.fill(&mut bytes);
    bytes[6] = (bytes[6] & 0x0f) | 0x40; // version 4
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // variant 10xx
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    )
}

/// Random date within the last ~10 years, as `YYYY-MM-DD`. Not a real
/// calendar walk (no month-length awareness) — day-of-month is capped
/// at 28 specifically so every combination is guaranteed valid without
/// needing a date library dependency just for this.
fn random_date(rng: &mut StdRng) -> String {
    let year = rng.gen_range(2016..=2026);
    let month = rng.gen_range(1..=12);
    let day = rng.gen_range(1..=28);
    format!("{year:04}-{month:02}-{day:02}")
}

fn random_datetime(rng: &mut StdRng) -> String {
    let date = random_date(rng);
    let (h, m, s) = (
        rng.gen_range(0..24),
        rng.gen_range(0..60),
        rng.gen_range(0..60),
    );
    format!("{date}T{h:02}:{m:02}:{s:02}Z")
}

fn generate_value(col: &Column, row_index: usize, rng: &mut StdRng) -> Result<Value, String> {
    Ok(match col.kind {
        ColumnType::Id => json!(row_index + 1),
        ColumnType::Uuid => json!(random_uuid_v4(rng)),
        ColumnType::Name => json!(Name().fake_with_rng::<String, _>(rng)),
        ColumnType::Email => json!(FreeEmail().fake_with_rng::<String, _>(rng)),
        ColumnType::Int => {
            let lo = col.min.unwrap_or(0.0) as i64;
            let hi = col.max.unwrap_or(1000.0) as i64;
            json!(rng.gen_range(lo..=hi))
        }
        ColumnType::Float => {
            let lo = col.min.unwrap_or(0.0);
            let hi = col.max.unwrap_or(100.0);
            let v = rng.gen_range(lo..=hi);
            json!((v * 100.0).round() / 100.0)
        }
        ColumnType::Bool => json!(rng.gen_bool(0.5)),
        ColumnType::Date => json!(random_date(rng)),
        ColumnType::Datetime => json!(random_datetime(rng)),
        ColumnType::Word => json!(Word().fake_with_rng::<String, _>(rng)),
        ColumnType::Sentence => json!(Sentence(3..8).fake_with_rng::<String, _>(rng)),
        ColumnType::Phone => json!(PhoneNumber().fake_with_rng::<String, _>(rng)),
        ColumnType::City => json!(CityName().fake_with_rng::<String, _>(rng)),
        ColumnType::Country => json!(CountryName().fake_with_rng::<String, _>(rng)),
        ColumnType::Enum => {
            let values = col
                .values
                .as_ref()
                .ok_or_else(|| format!("column '{}': enum with no values", col.name))?;
            let idx = rng.gen_range(0..values.len());
            json!(values[idx])
        }
    })
}

/// Generates `count` rows (schema's own count if `count` is `None`),
/// each an ordered `name -> value` map matching column declaration
/// order, driven by `rng` — callers control determinism entirely
/// through which `StdRng` they pass in.
pub fn generate_rows(
    schema: &Schema,
    count: Option<usize>,
    rng: &mut StdRng,
) -> Result<Vec<Map<String, Value>>, String> {
    let n = count.unwrap_or(schema.count);
    let mut rows = Vec::with_capacity(n);
    for row_index in 0..n {
        let mut row = Map::new();
        for col in &schema.columns {
            row.insert(col.name.clone(), generate_value(col, row_index, rng)?);
        }
        rows.push(row);
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::parse_schema;
    use rand::SeedableRng;

    fn rng() -> StdRng {
        StdRng::seed_from_u64(42)
    }

    #[test]
    fn generates_the_requested_row_count() {
        let schema = parse_schema("count: 7\ncolumns:\n  - name: id\n    type: id\n").unwrap();
        let rows = generate_rows(&schema, None, &mut rng()).unwrap();
        assert_eq!(rows.len(), 7);
    }

    #[test]
    fn explicit_count_overrides_schema_count() {
        let schema = parse_schema("count: 7\ncolumns:\n  - name: id\n    type: id\n").unwrap();
        let rows = generate_rows(&schema, Some(3), &mut rng()).unwrap();
        assert_eq!(rows.len(), 3);
    }

    #[test]
    fn id_column_is_sequential_starting_at_one() {
        let schema = parse_schema("count: 3\ncolumns:\n  - name: id\n    type: id\n").unwrap();
        let rows = generate_rows(&schema, None, &mut rng()).unwrap();
        let ids: Vec<i64> = rows.iter().map(|r| r["id"].as_i64().unwrap()).collect();
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn int_column_respects_min_and_max() {
        let schema = parse_schema(
            "count: 200\ncolumns:\n  - name: age\n    type: int\n    min: 18\n    max: 21\n",
        )
        .unwrap();
        let rows = generate_rows(&schema, None, &mut rng()).unwrap();
        for r in &rows {
            let age = r["age"].as_i64().unwrap();
            assert!((18..=21).contains(&age), "age {age} out of range");
        }
    }

    #[test]
    fn enum_column_only_produces_declared_values() {
        let schema = parse_schema(
            "count: 100\ncolumns:\n  - name: status\n    type: enum\n    values: [a, b]\n",
        )
        .unwrap();
        let rows = generate_rows(&schema, None, &mut rng()).unwrap();
        for r in &rows {
            let v = r["status"].as_str().unwrap();
            assert!(v == "a" || v == "b");
        }
    }

    #[test]
    fn same_seed_produces_identical_output() {
        let schema = parse_schema(
            "count: 5\ncolumns:\n  - name: name\n    type: name\n  - name: email\n    type: email\n",
        )
        .unwrap();
        let rows_a = generate_rows(&schema, None, &mut StdRng::seed_from_u64(7)).unwrap();
        let rows_b = generate_rows(&schema, None, &mut StdRng::seed_from_u64(7)).unwrap();
        assert_eq!(rows_a, rows_b);
    }

    #[test]
    fn uuid_looks_like_a_v4_uuid() {
        let schema = parse_schema("count: 1\ncolumns:\n  - name: id\n    type: uuid\n").unwrap();
        let rows = generate_rows(&schema, None, &mut rng()).unwrap();
        let id = rows[0]["id"].as_str().unwrap();
        assert_eq!(id.len(), 36);
        assert_eq!(id.chars().nth(14), Some('4'));
    }
}
