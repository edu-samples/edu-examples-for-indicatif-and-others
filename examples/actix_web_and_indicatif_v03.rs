use actix_web::{web, App, HttpRequest, HttpResponse, HttpServer, Responder, middleware};
use serde::Serialize;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use actix_web::web::Bytes;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::Arc;
use rand::seq::SliceRandom;
use std::time::Duration;

const REQUEST_ITERATIONS: usize = 40;
const ITERATION_TIME_MS: u64 = 100;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let multi_progress = Arc::new(MultiProgress::new());    
    let app_state = web::Data::new(AppState {
        multi_progress: multi_progress.clone(),
    });


    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            .wrap(middleware::Logger::default())
            .default_service(web::to(handle_request))
    })
    .bind(("0.0.0.0", 11555))?
    .run()
    .await
}

async fn handle_request(
    req: HttpRequest,
    data: web::Data<AppState>,
) -> impl Responder {
    let process_id = get_next_process_id();

    let (tx, rx) = mpsc::channel::<Result<Bytes, actix_web::Error>>(100);
    let data_clone = data.clone();

    spawn_new_process(
        data_clone,
        process_id,
        req.clone(),
        tx,
    );

    let response_stream = ReceiverStream::new(rx);

    HttpResponse::Ok()
        .content_type("application/jsonl")
        .streaming(response_stream)
}

use tokio::sync::mpsc::Sender;

#[derive(Serialize)]
struct StatusMessage {
    id: usize,
    status: String,
}

fn spawn_new_process(
    data: web::Data<AppState>,
    process_id: usize,
    _req: HttpRequest,
    tx: Sender<Result<Bytes, actix_web::Error>>,
) {
    let pb = data.multi_progress.add(ProgressBar::new(REQUEST_ITERATIONS as u64));
    pb.set_style(
        ProgressStyle::with_template("{prefix:.bold.dim} {spinner} {wide_msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    pb.set_prefix(format!("[{}/?]", process_id));


    let pb_clone = pb.clone();

    actix_web::rt::spawn(async move {
        let commands = ["building", "compiling", "testing", "deploying"];
        let packages = ["moduleA", "moduleB", "moduleC"];

        for _ in 0..REQUEST_ITERATIONS {
            tokio::time::sleep(Duration::from_millis(ITERATION_TIME_MS)).await;
            // Create RNG inside the loop to avoid Send issues
            let mut rng = rand::thread_rng();
            let cmd = *commands.choose(&mut rng).unwrap();
            let pkg = *packages.choose(&mut rng).unwrap();
            pb_clone.set_message(format!("{}: {}", pkg, cmd));
            pb_clone.inc(1);
            let status_message = StatusMessage {
                id: process_id,
                status: format!("{}: {}", pkg, cmd),
            };
            let json_line = serde_json::to_string(&status_message).unwrap() + "\n";
            if tx.send(Ok(Bytes::from(json_line))).await.is_err() {
                break;
            }
        }
        pb_clone.finish_with_message("Done");

        let final_message = StatusMessage {
            id: process_id,
            status: "Done".to_string(),
        };
        let json_line = serde_json::to_string(&final_message).unwrap() + "\n";
        let _ = tx.send(Ok(Bytes::from(json_line))).await;
    });
}

struct AppState {
    multi_progress: Arc<MultiProgress>,
}

static PROCESS_ID_COUNTER: AtomicUsize = AtomicUsize::new(1);

fn get_next_process_id() -> usize {
    PROCESS_ID_COUNTER.fetch_add(1, Ordering::SeqCst)
}
