use actix_service::Service;
use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use opentelemetry::api::trace::futures::FutureExt;
use opentelemetry::api::{Key, TraceContextExt, Tracer};
use opentelemetry::{global, sdk};
use std::error::Error;

fn init_tracer() -> Result<sdk::Tracer, Box<dyn Error>> {
    opentelemetry_jaeger::new_pipeline()
        .with_agent_endpoint("localhost:6831")
        .with_service_name("trace-udp-demo")
        .install()
}

async fn index() -> &'static str {
    let tracer = global::tracer("request");
    tracer.in_span("index", |ctx| {
        ctx.span().set_attribute(Key::new("parameter").i64(10));
        "Index"
    })
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();
    init_tracer().expect("Failed to initialise tracer.");

    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .wrap_fn(|req, srv| {
                let tracer = global::tracer("request");
                tracer.in_span("middleware", move |cx| {
                    cx.span().set_attribute(Key::new("path").string(req.path()));
                    srv.call(req).with_context(cx)
                })
            })
            .route("/", web::get().to(index))
    })
    .bind("127.0.0.1:8088")
    .unwrap()
    .run()
    .await
}
