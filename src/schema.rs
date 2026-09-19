use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Schema {
    #[serde(default = "default_table")]
    pub table: String,
    #[serde(default = "default_count")]
    pub count: usize,
    pub columns: Vec<Column>,
}

fn default_table() -> String {
    "rows".to_string()
}
fn default_count() -> usize {
    10
}

#[derive(Debug, Deserialize)]
pub struct Column {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: ColumnType,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub values: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ColumnType {
    Id,
    Uuid,
    Name,
    Email,
    Int,
    Float,
    Bool,
    Date,
    Datetime,
    Word,
    Sentence,
    Phone,
    City,
    Country,
    Enum,
}

/// Parses a schema from YAML or JSON — JSON is valid YAML, so one
/// parser (`serde_yaml`) handles both; the caller doesn't need to pick.
pub fn parse_schema(text: &str) -> Result<Schema, String> {
    let schema: Schema = serde_yaml::from_str(text).map_err(|e| format!("invalid schema: {e}"))?;
    if schema.columns.is_empty() {
        return Err("schema has no columns".to_string());
    }
    for col in &schema.columns {
        if col.kind == ColumnType::Enum && col.values.as_ref().is_none_or(|v| v.is_empty()) {
            return Err(format!(
                "column '{}' is type 'enum' but has no 'values' list",
                col.name
            ));
        }
        if let (Some(min), Some(max)) = (col.min, col.max) {
            if min > max {
                return Err(format!(
                    "column '{}': min ({min}) is greater than max ({max})",
                    col.name
                ));
            }
        }
    }
    Ok(schema)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_realistic_yaml_schema() {
        let text = r#"
table: users
count: 5
columns:
  - name: id
    type: id
  - name: email
    type: email
  - name: age
    type: int
    min: 18
    max: 65
  - name: status
    type: enum
    values: [active, banned]
"#;
        let schema = parse_schema(text).unwrap();
        assert_eq!(schema.table, "users");
        assert_eq!(schema.count, 5);
        assert_eq!(schema.columns.len(), 4);
    }

    #[test]
    fn parses_json_too_since_json_is_valid_yaml() {
        let text = r#"{"table": "t", "columns": [{"name": "id", "type": "id"}]}"#;
        let schema = parse_schema(text).unwrap();
        assert_eq!(schema.table, "t");
    }

    #[test]
    fn defaults_table_and_count_when_omitted() {
        let text = "columns:\n  - name: id\n    type: id\n";
        let schema = parse_schema(text).unwrap();
        assert_eq!(schema.table, "rows");
        assert_eq!(schema.count, 10);
    }

    #[test]
    fn enum_without_values_is_a_clean_error() {
        let text = "columns:\n  - name: status\n    type: enum\n";
        assert!(parse_schema(text).is_err());
    }

    #[test]
    fn min_greater_than_max_is_a_clean_error() {
        let text = "columns:\n  - name: age\n    type: int\n    min: 50\n    max: 10\n";
        assert!(parse_schema(text).is_err());
    }

    #[test]
    fn no_columns_is_a_clean_error() {
        assert!(parse_schema("columns: []").is_err());
    }
}
