use env_logger::Env;
use std::net::TcpListener;

use sqlx::PgPool;
use zero2prod::{configuration::get_configuration, startup::run};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // set up the logger. If the RUST_LOG env variable is set then it uses that level of logging
    // otherwise it defaults to info.
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let configuration = get_configuration().expect("Failed to read configuration");
    let address = format!("127.0.0.1:{}", configuration.application_port);

    let connection_pool = PgPool::connect(&configuration.database.get_connection_string())
        .await
        .expect("Failed to connect to postgres");
    let listener = TcpListener::bind(address)?;

    run(listener, connection_pool)?.await
}
