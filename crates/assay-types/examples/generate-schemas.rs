//! Generator binary that iterates the schema registry and writes JSON Schema files.
//!
//! Run via: `cargo run -p assay-types --example generate-schemas`
//! Or via:  `just schemas`

use assay_types::schema_registry;
use std::fs;
use std::path::Path;

fn main() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("navigate to crates/")
        .parent()
        .expect("navigate to workspace root");
    let out_dir = workspace_root.join("schemas");

    fs::create_dir_all(&out_dir).expect("create schemas/ directory");

    let mut count = 0;
    for entry in schema_registry::all_entries() {
        let mut schema = (entry.generate)();
        schema.insert(
            "$id".to_owned(),
            format!("https://assay.dev/schemas/{}.schema.json", entry.name).into(),
        );

        let json = serde_json::to_string_pretty(&schema).expect("serialize schema to JSON");
        let path = out_dir.join(format!("{}.schema.json", entry.name));
        fs::write(&path, format!("{json}\n")).expect("write schema file");
        println!("  wrote {}", path.display());
        count += 1;
    }

    println!("\nGenerated {count} schema files in {}", out_dir.display());
}
