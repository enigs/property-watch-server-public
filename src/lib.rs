use std::{future::Future, path::Path, time::Duration};

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{Request, Response, Status, transport::Server};

pub mod proto {
    tonic::include_proto!("propertywatch.v1");
}

use proto::{
    HelloReply, HelloRequest,
    greeter_server::{Greeter, GreeterServer},
};

pub async fn open_database(path: &Path) -> Result<SqlitePool, sqlx::Error> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(Duration::from_secs(5));
    let pool = SqlitePoolOptions::new()
        .max_connections(2)
        .connect_with(options)
        .await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS service_metadata (key TEXT PRIMARY KEY NOT NULL, value TEXT NOT NULL) STRICT")
        .execute(&pool).await?;
    sqlx::query("INSERT INTO service_metadata (key,value) VALUES ('service','property-watch') ON CONFLICT(key) DO NOTHING")
        .execute(&pool).await?;
    Ok(pool)
}

struct HelloService {
    pool: SqlitePool,
}

#[tonic::async_trait]
impl Greeter for HelloService {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        sqlx::query("SELECT value FROM service_metadata WHERE key='service'")
            .fetch_one(&self.pool)
            .await
            .map_err(|_| Status::unavailable("Database is unavailable"))?;
        let request = request.into_inner();
        let name = request.name.trim();
        let name = if name.is_empty() { "World" } else { name };
        Ok(Response::new(HelloReply {
            message: format!("Hello, {name}!"),
        }))
    }
}

pub async fn serve(
    listener: TcpListener,
    pool: SqlitePool,
    shutdown: impl Future<Output = ()> + Send + 'static,
) -> Result<(), tonic::transport::Error> {
    let (reporter, health) = tonic_health::server::health_reporter();
    reporter.set_serving::<GreeterServer<HelloService>>().await;
    Server::builder()
        .timeout(Duration::from_secs(10))
        .add_service(health)
        .add_service(GreeterServer::new(HelloService { pool }).max_decoding_message_size(1024))
        .serve_with_incoming_shutdown(TcpListenerStream::new(listener), shutdown)
        .await
}
