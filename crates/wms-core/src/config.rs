use std::path::Path;

// CARGO_MANIFEST_DIR is baked in at compile time: it resolves on the host, and
// inside a container it points nowhere, so both loads fail quietly and the real
// environment variables set by the orchestrator win. Same code either way.
pub fn load_dotenv(manifest_dir: &str) {
    let _ = dotenvy::from_path(Path::new(manifest_dir).join(".env"));
    let _ = dotenvy::dotenv();
}

pub fn env_var(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| panic!("{key} must be set"))
}

pub fn env_var_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

pub fn env_parse_or<T>(key: &str, default: T) -> T
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match std::env::var(key) {
        Ok(raw) => raw
            .parse()
            .unwrap_or_else(|err| panic!("{key} is not a valid value: {err}")),
        Err(_) => default,
    }
}
