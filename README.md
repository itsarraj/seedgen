# seedgen

Generates realistic fake data — CSV, JSON, or SQL `INSERT`s — from a
small YAML/JSON schema. Rust's `fake` crate is a solid *library* for
this, but there's no CLI wrapper around it: today, generating a batch
of realistic seed rows means writing a throwaway Rust (or Python
`faker`, or Node `@faker-js/faker`) script every time. This is that
script, generalized into a schema file you write once and reuse.

## Installation

```sh
cargo install --path .
```

## Usage

```bash
seedgen schema.yaml                        # CSV to stdout, using schema's own row count
seedgen schema.yaml --format json -n 50     # JSON, 50 rows regardless of schema's count
seedgen schema.yaml --format sql --seed 42  # SQL INSERTs, reproducible across runs
```

Schema file:

```yaml
table: users
count: 5
columns:
  - name: id
    type: id          # sequential integer, starting at 1
  - name: uuid
    type: uuid
  - name: name
    type: name
  - name: email
    type: email
  - name: age
    type: int
    min: 18
    max: 65
  - name: signup_date
    type: date
  - name: active
    type: bool
  - name: plan
    type: enum
    values: [free, pro, enterprise]
```

Supported types: `id`, `uuid`, `name`, `email`, `int` (`min`/`max`),
`float` (`min`/`max`), `bool`, `date`, `datetime`, `word`, `sentence`,
`phone`, `city`, `country`, `enum` (`values`). JSON is valid YAML, so a
`.json` schema works with the same parser, no separate flag needed.

## Status: built and verified, including a real cross-format bug caught by comparing outputs against each other

- **19 unit tests**: schema parsing and validation (enum with no
  `values`, `min > max`, missing columns — all clean errors, not
  panics), row generation (`--seed` determinism, `id` sequencing,
  `min`/`max` bounds actually respected across 200 generated rows,
  `enum` never producing an undeclared value, UUID v4 version-nibble
  correctness), and all three output formats (CSV comma/quote
  escaping, SQL string escaping and empty-row handling, JSON validity).
- **Live-run against a realistic 8-column schema** in all three
  formats with a fixed `--seed`, and the CSV/JSON/SQL outputs were
  cross-checked against each other by eye — same UUIDs, same names,
  same ages, same enum values in every format, run for run. A second
  run with the same seed produced byte-identical output, confirmed
  with `diff`.
- **A real bug this comparison caught**: the JSON output was silently
  emitting columns in **alphabetical order** (`active, age, email, id,
  name, plan, signup_date, uuid`) while CSV and SQL correctly followed
  the schema's declared order (`id, uuid, name, email, age,
  signup_date, active, plan`) — because `serde_json::Map` sorts its
  keys by default, and only the CSV/SQL paths were explicitly
  reordering by the schema's column list before formatting; the JSON
  path was serializing the map directly. Fixed by enabling
  `serde_json`'s `preserve_order` feature (see `Cargo.toml`) so
  insertion order — which is schema order, since rows are built column
  by column — is what actually gets serialized. Locked down with a
  regression test (`json_key_order_follows_insertion_order_not_alphabetical`)
  using a column pair (`zeta` before `alpha`) specifically chosen so
  alphabetical sorting and insertion order would visibly disagree.
- A schema with an unknown column `type` fails with a specific,
  readable error naming every valid type — verified live, not just
  asserted in a test.

**Deliberate scope limits, stated plainly**: dates are generated
without real calendar awareness (day-of-month capped at 28 so every
combination is valid without pulling in a date library just for this);
no relational integrity between generated tables (a `user_id` foreign
key in one schema won't reference real `id`s generated from another —
each schema file is generated independently); locale is English-only
(whatever `fake`'s `en` locale produces).
