use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use listenfd::ListenFd; // listen to the socket of systemfd
use std::sync::Mutex;
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

async fn index2() -> impl Responder {
    HttpResponse::Ok().body("Hello world again!")
}

// #[get("/hello")]
async fn api_index() -> impl Responder {
    HttpResponse::Ok().body("API index here!")
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
            .route("/api", web::get().to(api_index))
            .service(
                web::scope("/api")
                    .route("/count", web::get().to(count))
                    .route("/hello", web::get().to(index2)),
            )
    });

    server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
        server.listen(l)?
    } else {
        server.bind("127.0.0.1:3000")?
    };

    server.run().await
}
