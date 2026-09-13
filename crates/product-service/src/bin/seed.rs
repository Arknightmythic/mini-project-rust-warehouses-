use std::path::{Path, PathBuf};

use wms_core::db::DbConfig;

// CARGO_MANIFEST_DIR is baked in at compile time, so it resolves on the host but
// points at a nonexistent path inside the container — where ./seeders is correct.
fn seeders_dir() -> PathBuf {
    let in_crate = Path::new(env!("CARGO_MANIFEST_DIR")).join("seeders");
    if in_crate.is_dir() {
        in_crate
    } else {
        PathBuf::from("seeders")
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    wms_core::config::load_dotenv(env!("CARGO_MANIFEST_DIR"));

    let pool = wms_core::db::connect(&DbConfig::from_env()).await?;

    let mut entries: Vec<_> = std::fs::read_dir(seeders_dir())?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "sql"))
        .collect();

    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let file_name = path.file_name().unwrap().to_string_lossy().to_string();
        let sql = std::fs::read_to_string(&path)?;

        println!("Running seeder: {file_name}");
        sqlx::raw_sql(sqlx::AssertSqlSafe(sql))
            .execute(&pool)
            .await?;
    }

    println!("All seeders executed successfully.");
    Ok(())
}
