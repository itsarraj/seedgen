use std::fs;
use std::path::PathBuf;

use clap::{Parser, ValueEnum};
use rand::rngs::StdRng;
use rand::SeedableRng;

use seedgen::generate::generate_rows;
use seedgen::output::{to_csv, to_json, to_sql};
use seedgen::schema::parse_schema;

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Csv,
    Json,
    Sql,
}

#[derive(Parser)]
#[command(
    name = "seedgen",
    about = "Generates realistic fake data (CSV/JSON/SQL) from a small YAML/JSON schema"
)]
struct Cli {
    /// Schema file (YAML or JSON) describing the table and its columns.
    schema: PathBuf,

    /// Output format.
    #[arg(short, long, value_enum, default_value = "csv")]
    format: Format,

    /// Row count — overrides the schema's own `count` if given.
    #[arg(short = 'n', long)]
    count: Option<usize>,

    /// RNG seed, for reproducible output across runs.
    #[arg(long)]
    seed: Option<u64>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let text = fs::read_to_string(&cli.schema)
        .map_err(|e| anyhow::anyhow!("reading {}: {e}", cli.schema.display()))?;
    let schema =
        parse_schema(&text).map_err(|e| anyhow::anyhow!("{}: {e}", cli.schema.display()))?;

    let mut rng = match cli.seed {
        Some(s) => StdRng::seed_from_u64(s),
        None => StdRng::from_entropy(),
    };
    let rows = generate_rows(&schema, cli.count, &mut rng).map_err(|e| anyhow::anyhow!(e))?;

    let columns: Vec<String> = schema.columns.iter().map(|c| c.name.clone()).collect();
    let output = match cli.format {
        Format::Csv => to_csv(&rows, &columns),
        Format::Json => to_json(&rows),
        Format::Sql => to_sql(&schema.table, &rows, &columns),
    };
    print!("{output}");

    Ok(())
}
