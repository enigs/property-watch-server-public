use std::{env, error::Error, path::Path, time::Duration};

use property_watch_server::{
    open_database,
    proto::{HelloRequest, greeter_client::GreeterClient},
    serve,
};
use tokio::net::TcpListener;
use tonic::transport::Endpoint;
use tonic_health::pb::{
    HealthCheckRequest, health_check_response::ServingStatus, health_client::HealthClient,
};

#[tokio::main(worker_threads = 2)]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("serve") {
        "serve" => {
            let path =
                env::var("PW_DATABASE_PATH").unwrap_or_else(|_| "property-watch.sqlite3".into());
            let bind = env::var("PW_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:50061".into());
            let pool = open_database(Path::new(&path)).await?;
            let listener = TcpListener::bind(&bind).await?;
            eprintln!(
                "Property Watch gRPC listening on {}",
                listener.local_addr()?
            );
            let result = serve(listener, pool.clone(), shutdown()).await;
            pool.close().await;
            result?;
        }
        "check" | "hello" => {
            let endpoint = args
                .get(2)
                .map(String::as_str)
                .unwrap_or("http://127.0.0.1:50061");
            let channel = Endpoint::from_shared(endpoint.to_owned())?
                .connect_timeout(Duration::from_secs(5))
                .timeout(Duration::from_secs(5))
                .connect()
                .await?;
            let status = HealthClient::new(channel.clone())
                .check(HealthCheckRequest {
                    service: "propertywatch.v1.Greeter".into(),
                })
                .await?
                .into_inner();
            if status.status != ServingStatus::Serving as i32 {
                return Err("gRPC service is not serving".into());
            }
            let name = args.get(3).cloned().unwrap_or_else(|| "World".into());
            let reply = GreeterClient::new(channel)
                .say_hello(HelloRequest { name })
                .await?
                .into_inner();
            println!("{}", reply.message);
        }
        _ => return Err(
            "Usage: pw-server [serve | check [http://endpoint] | hello [http://endpoint] [name]]"
                .into(),
        ),
    }
    Ok(())
}

async fn shutdown() {
    let mut terminate = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .expect("install SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {},
        _ = terminate.recv() => {},
    }
}
