use sqlx::postgres::PgPoolOptions;
use std::path::{Path, PathBuf};

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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

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
