use ::SimpleProxy::middlewares::logger_middleware_test::Logger;
use ::SimpleProxy::{Environment, SimpleProxy};


#[tokio::main]
async fn main() {
    let mut proxy = SimpleProxy::new(12345, Environment::Development);
    let logger = Logger::new();

    proxy.add_middleware(Box::new(logger));
    let _ = proxy.run().await;
}