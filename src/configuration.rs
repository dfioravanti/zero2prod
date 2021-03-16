use config::{Config, ConfigError, File};
#[derive(serde::Deserialize)]
pub struct Setting {
    pub database: DatabaseSettings,
    pub application_port: u16,
}

pub fn get_configuration() -> Result<Setting, ConfigError> {
    let mut settings = Config::default();
    settings.merge(File::with_name("configuration"))?;
    settings.try_into()
}

#[derive(serde::Deserialize)]
pub struct DatabaseSettings {
    pub username: String,
    pub password: String,
    pub port: u16,
    pub host: String,
    pub database_name: String,
}

impl DatabaseSettings {
    /// Returns the connection string for the stored database
    pub fn get_connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            self.username, self.password, self.host, self.port, self.database_name
        )
    }
    /// Returns the connection string for the default database
    pub fn get_connection_string_default_db(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}",
            self.username, self.password, self.host, self.port
        )
    }
}
