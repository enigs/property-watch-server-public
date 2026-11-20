use property_watch_server::{
    open_database,
    proto::{HelloRequest, greeter_client::GreeterClient},
    serve,
};
use tokio::net::TcpListener;
use tonic_health::pb::{
    HealthCheckRequest, health_check_response::ServingStatus, health_client::HealthClient,
};

#[tokio::test]
async fn grpc_uses_a_persistent_isolated_database() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("property-watch.sqlite3");
    let pool = open_database(&path).await.unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let server_pool = pool.clone();
    let task = tokio::spawn(serve(listener, server_pool, async {
        let _ = stopped.await;
    }));
    let mut client = GreeterClient::connect(endpoint.clone()).await.unwrap();
    for (name, expected) in [("", "Hello, World!"), ("Scout", "Hello, Scout!")] {
        assert_eq!(
            client
                .say_hello(HelloRequest { name: name.into() })
                .await
                .unwrap()
                .into_inner()
                .message,
            expected
        );
    }
    let channel = tonic::transport::Endpoint::from_shared(endpoint)
        .unwrap()
        .connect()
        .await
        .unwrap();
    let health = HealthClient::new(channel)
        .check(HealthCheckRequest {
            service: "propertywatch.v1.Greeter".into(),
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(health.status, ServingStatus::Serving as i32);
    let too_large = client
        .say_hello(HelloRequest {
            name: "x".repeat(2048),
        })
        .await
        .unwrap_err();
    assert_eq!(too_large.code(), tonic::Code::OutOfRange);
    sqlx::query("INSERT INTO service_metadata VALUES ('persistent-test','kept')")
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
    assert_eq!(
        client
            .say_hello(HelloRequest {
                name: "Scout".into()
            })
            .await
            .unwrap_err()
            .code(),
        tonic::Code::Unavailable
    );
    stop.send(()).unwrap();
    task.await.unwrap().unwrap();
    let reopened = open_database(&path).await.unwrap();
    let value: String =
        sqlx::query_scalar("SELECT value FROM service_metadata WHERE key='persistent-test'")
            .fetch_one(&reopened)
            .await
            .unwrap();
    assert_eq!(value, "kept");
    reopened.close().await;
}
