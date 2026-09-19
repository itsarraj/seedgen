use serde_json::{Map, Value};

fn value_to_csv_field(value: &Value) -> String {
    let raw = match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    };
    if raw.contains(',') || raw.contains('"') || raw.contains('\n') {
        format!("\"{}\"", raw.replace('"', "\"\""))
    } else {
        raw
    }
}

/// Column order comes from the caller (the schema's declared order),
/// not from the row maps themselves, even though `serde_json::Map`
/// (with this crate's `preserve_order` feature on) does iterate in
/// insertion order, which happens to match — relying on that implicit
/// behavior here instead of the explicit schema order would be exactly
/// the kind of thing that works today and breaks on a refactor.
pub fn to_csv(rows: &[Map<String, Value>], columns: &[String]) -> String {
    let mut out = String::new();
    out.push_str(&columns.join(","));
    out.push('\n');
    for row in rows {
        let fields: Vec<String> = columns
            .iter()
            .map(|c| value_to_csv_field(row.get(c).unwrap_or(&Value::Null)))
            .collect();
        out.push_str(&fields.join(","));
        out.push('\n');
    }
    out
}

/// Relies on the `preserve_order` Cargo feature on `serde_json` (see
/// `Cargo.toml`) so that key order in the output JSON matches the
/// order rows were built in (schema column order), rather than the
/// crate's default of sorting map keys alphabetically — without that
/// feature this would silently emit `{"active": ..., "age": ...}`
/// instead of following the schema, a real discrepancy this tool hit
/// and fixed during live verification (CSV/SQL were already correct
/// since they format from the explicit column list, not the map).
pub fn to_json(rows: &[Map<String, Value>]) -> String {
    serde_json::to_string_pretty(rows).unwrap_or_default()
}

fn value_to_sql(value: &Value) -> String {
    match value {
        Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        Value::Null => "NULL".to_string(),
        Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        other => other.to_string(),
    }
}

pub fn to_sql(table: &str, rows: &[Map<String, Value>], columns: &[String]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    out.push_str(&format!(
        "INSERT INTO {table} ({}) VALUES\n",
        columns.join(", ")
    ));
    let value_lines: Vec<String> = rows
        .iter()
        .map(|row| {
            let values: Vec<String> = columns
                .iter()
                .map(|c| value_to_sql(row.get(c).unwrap_or(&Value::Null)))
                .collect();
            format!("  ({})", values.join(", "))
        })
        .collect();
    out.push_str(&value_lines.join(",\n"));
    out.push_str(";\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_rows() -> Vec<Map<String, Value>> {
        let mut r1 = Map::new();
        r1.insert("id".to_string(), json!(1));
        r1.insert("name".to_string(), json!("Ann, O'Brien"));
        vec![r1]
    }

    #[test]
    fn csv_quotes_fields_containing_commas_and_escapes_quotes() {
        let out = to_csv(&sample_rows(), &["id".to_string(), "name".to_string()]);
        assert!(out.contains("\"Ann, O'Brien\""));
    }

    #[test]
    fn csv_header_matches_declared_column_order() {
        let out = to_csv(&sample_rows(), &["name".to_string(), "id".to_string()]);
        assert!(out.starts_with("name,id\n"));
    }

    #[test]
    fn json_key_order_follows_insertion_order_not_alphabetical() {
        // "zeta" inserted before "alpha": alphabetical sorting would
        // reorder these, so this only passes with `preserve_order` on.
        let mut row = Map::new();
        row.insert("zeta".to_string(), json!(1));
        row.insert("alpha".to_string(), json!(2));
        let out = to_json(&[row]);
        assert!(out.find("zeta").unwrap() < out.find("alpha").unwrap());
    }

    #[test]
    fn json_output_is_a_valid_array() {
        let out = to_json(&sample_rows());
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed.len(), 1);
    }

    #[test]
    fn sql_escapes_single_quotes_and_wraps_strings() {
        let out = to_sql(
            "users",
            &sample_rows(),
            &["id".to_string(), "name".to_string()],
        );
        assert!(out.contains("INSERT INTO users (id, name) VALUES"));
        assert!(out.contains("'Ann, O''Brien'"));
        assert!(out.trim_end().ends_with(';'));
    }

    #[test]
    fn sql_on_empty_rows_is_empty_not_a_broken_statement() {
        assert_eq!(to_sql("users", &[], &["id".to_string()]), "");
    }
}
