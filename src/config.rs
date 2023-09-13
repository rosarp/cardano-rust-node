use figment::{
    providers::{Format, Yaml},
    Figment,
};
use serde::Deserialize;
use tracing_subscriber::fmt::format::FmtSpan;

#[derive(Debug, PartialEq, Deserialize)]
pub struct AppConfig {
    pub hosts: Vec<HostConfig>,
    pub supported_versions: Vec<i64>,
    max_supported_version: u8,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct HostConfig {
    pub host: String,
    pub network_magic: u32,
    pub network_id: String,
}

pub fn enable_tracing() {
    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_file(false)
        .with_line_number(false)
        .with_thread_ids(true)
        .with_target(false)
        .with_span_events(FmtSpan::FULL)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();
}

pub fn get_app_config() -> AppConfig {
    let figment = Figment::new().merge(Yaml::file("App.yaml"));
    figment.extract().unwrap()
}
