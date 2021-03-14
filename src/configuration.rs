use config::{Config, ConfigError, File};
#[derive(serde::Deserialize)]
pub struct Setting {
    pub database: DatabaseSettings,
    pub application_port: u16,
}

#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: String,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

pub fn get_configuration() -> Result<Setting, ConfigError> {
    let mut settings = Config::default();
    settings.merge(File::with_name("configuration"))?;
    settings.try_into()
}
