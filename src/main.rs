use actix_web::{web, guard, App, HttpRequest, HttpResponse, HttpServer, Responder, Error};
use listenfd::ListenFd; // listen to the socket of systemfd
use serde::{Serialize};
use std::sync::Mutex;
use std::time::Duration;
use futures::future::{ready, Ready};
extern crate num_cpus;

struct AppState {
    counter: Mutex<i32>, // Mutex is necessary to mutate safely across threads
    app_name: String,
    logical_cpus: usize,
    physical_cpus: usize,
}

async fn count(data: web::Data<AppState>) -> String {
    let mut counter = data.counter.lock().unwrap(); // get counter's MutexGuard
    *counter += 1; // access counter inside MutexGuard

    // std::thread::sleep(Duration::from_secs(10)); // bad practice, thread is blocking
    tokio::time::delay_for(Duration::from_secs(10)).await;

    format!("mutex counter: {}", counter)
}

async fn index(data: web::Data<AppState>) -> impl Responder {
    let app_name = &data.app_name;
    let response = format!(
        "Hello {}! I have {} logical cpus, {} cpus",
        app_name, &data.logical_cpus, &data.physical_cpus
    );

    HttpResponse::Ok().body(response)
}

async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello world again!")
}

// #[get("/hello")]
async fn api_index() -> impl Responder {
    HttpResponse::Ok().body("API index here!")
}

// config for api
fn api_test(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/count")
            .guard(guard::Header("Host", "localhost:3000"))
            .route(web::get().to(count))
    );
}

// config for api
fn api_hello(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/hello")
            .route(web::get().to(hello))
    );
}

// config for user api
fn api_user(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/user")
            .route(web::get().to(user_json_sample))
    );
}

#[derive(Serialize)]
struct MyObj {
    name: &'static str,
}

impl Responder for MyObj {
    type Error = Error;
    type Future = Ready<Result<HttpResponse, Error>>;

    fn respond_to(self, _req: &HttpRequest) -> Self::Future {
        let body = serde_json::to_string(&self).unwrap();

        // create response and set content-type
        ready(Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(body)))
    }
}

async fn user_json_sample() -> impl Responder {
    MyObj { name: "chenxi" }
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let mut listenfd = ListenFd::from_env();
    let mut server = HttpServer::new(|| {
        println!("serving...");
        App::new()
            .data(AppState {
                counter: Mutex::new(0),
                app_name: String::from("papi"),
                logical_cpus: num_cpus::get(),
                physical_cpus: num_cpus::get_physical(),
            })
            .route("/", web::get().to(index))
            .service(
                web::scope("/api")
                    // other guards https://docs.rs/actix-web/2.0.0/actix_web/guard/index.html#functions
                    .guard(guard::Header("Host", "localhost:3000"))
                    .route("", web::get().to(api_index))
                    .configure(api_test)
                    .configure(api_hello)
                    .configure(api_user)
            )
    })
    .keep_alive(75) // set keep-alive to 75 seconds
    .workers(4); // start with 4 workers, but in same thread

    server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
        server.listen(l)?
    } else {
        server.bind("127.0.0.1:3000")?
    };

    server.run().await
}
