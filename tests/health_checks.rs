use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use tracing::subscriber::set_global_default;
use tracing_log::LogTracer;
use uuid::Uuid;
use zero2prod::{
    configuration::{get_configuration, DatabaseSettings},
    startup::run,
    telemetry::get_subscriber,
};

pub struct TestApp {
    pub address: String,
    pub db_config: DatabaseSettings,
    pub db_pool: PgPool,
}

// The logger must be initialized only once.
lazy_static::lazy_static! {
    static ref TRACING: () = {
        // We do not want to look at the logging for each test.
        // Setting TEST_LOG to true will print the logs otherwise they are suppressed.
        let filter = if std::env::var("TEST_LOG").is_ok() {"debug"} else {""};
        let subscriber = get_subscriber("test".into(), filter.into());
        // redirect logs to our logger and set up subscriber
        LogTracer::init().expect("Failed to set logger");
        set_global_default(subscriber).expect("Failed to set subscriber");
    };
}

/// Spawns the app as a background task using a random port.
/// This is done to avoid clashing with already existing applications.
/// The application is spawned as a background tokio task so that we can use it in testing
async fn spawn_app() -> TestApp {
    lazy_static::initialize(&TRACING);
    // We create and use a random db name so that it does not clash with production or other tests
    let mut configuration = get_configuration().expect("Failed to read configuration");
    configuration.database.database_name = Uuid::new_v4().to_string();
    let connection_pool = configure_database(&configuration.database).await;

    // Bind the app to a random port.
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind random port");
    let port = listener.local_addr().unwrap().port();
    let address = format!("http://127.0.0.1:{}", port);

    // Spawn the app as a background task
    let server = run(listener, connection_pool.clone()).expect("Failed to bind address");
    let _ = tokio::spawn(server);

    TestApp {
        address,
        db_config: configuration.database,
        db_pool: connection_pool,
    }
}

/// Clean up after we are done running the tests
async fn clean_up(app: TestApp) {
    app.db_pool.close().await;

    let mut connection = PgConnection::connect(&app.db_config.get_connection_string_default_db())
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(&*format!(
            r#"DROP DATABASE "{}";"#,
            &app.db_config.database_name
        ))
        .await
        .expect("Failed to create the test database");
}

pub async fn configure_database(config: &DatabaseSettings) -> PgPool {
    let mut connection = PgConnection::connect(&config.get_connection_string_default_db())
        .await
        .expect("Failed to connect to Postgres");

    connection
        .execute(&*format!(r#"CREATE DATABASE "{}";"#, config.database_name))
        .await
        .expect("Failed to create the test database");

    let connection_pool = PgPool::connect(&config.get_connection_string())
        .await
        .expect("Failed to connect to Postgres");

    sqlx::migrate!("./migrations")
        .run(&connection_pool)
        .await
        .expect("Failed to migrate the database");

    connection_pool
}

#[actix_rt::test]
async fn health_check_works() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/health_check", &app.address))
        .send()
        .await
        .expect("Failed to execute request.");

    assert!(response.status().is_success());
    assert_eq!(Some(0), response.content_length());

    clean_up(app).await;
}

#[actix_rt::test]
async fn subscribe_return_200_with_valid_data() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let body = "name=le%20guin&email=ursula_le_guin%40gmail.com";

    let response = client
        .post(&format!("{}/subscriptions", &app.address))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(200, response.status().as_u16());

    let saved = sqlx::query!("SELECT email, name FROM subscriptions",)
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved subscription");

    assert_eq!(saved.email, "ursula_le_guin@gmail.com");
    assert_eq!(saved.name, "le guin");

    clean_up(app).await;
}

#[actix_rt::test]
async fn subscribe_return_400_with_missing_data() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();
    let test_cases = vec![
        ("name=le%20guin", "missing the email"),
        ("email=ursula_le_guin%40gmail.com", "missing the name"),
        ("", "missing both the name and email"),
    ];

    for (invalid_body, error_message) in test_cases {
        let response = client
            .post(&format!("{}/subscriptions", &app.address))
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(invalid_body)
            .send()
            .await
            .expect("Failed to execute request");

        assert_eq!(
            400,
            response.status().as_u16(),
            "The API did not return 400 when the payload was {}",
            error_message
        );
    }

    clean_up(app).await;
}
