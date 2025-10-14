use anyhow::Context;
use dotenvy::var;

pub fn get_var(key: &str) -> anyhow::Result<String> {
    var(key).context(format!("{key} is not set"))
}
