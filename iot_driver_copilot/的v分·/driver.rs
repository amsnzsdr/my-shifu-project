use actix_web::{web, App, HttpResponse, HttpServer, Responder, Result};
use actix_web::http::header;
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::Mutex;
use std::collections::VecDeque;
use std::time::{Duration, SystemTime};
use actix_web::middleware::Logger;

// ========== Static Device Info ==========
#[derive(Serialize)]
struct DeviceInfo {
    device_name: &'static str,
    device_model: &'static str,
    manufacturer: &'static str,
    device_type: &'static str,
}

// ========== CSV Data Point Model ==========
#[derive(Clone)]
struct CsvData {
    headers: Vec<&'static str>,
    rows: VecDeque<Vec<String>>,
}

impl CsvData {
    fn latest_csv(&self) -> String {
        let mut csv = self.headers.join(",") + "\n";
        if let Some(row) = self.rows.back() {
            csv += &row.join(",");
        }
        csv
    }
}

// ========== Command Model ==========
#[derive(Deserialize)]
struct CommandRequest {
    command: String,
    params: Option<serde_json::Value>,
}

// ========== State ==========
struct AppState {
    device_info: DeviceInfo,
    csv_data: Mutex<CsvData>,
}

// ========== ENV Utility ==========
fn get_env(key: &str, default: &str) -> String {
    env::var(key).unwrap_or_else(|_| default.to_owned())
}

// ========== Handlers ==========

// GET /info
async fn info(data: web::Data<AppState>) -> impl Responder {
    HttpResponse::Ok().json(&data.device_info)
}

// GET /data
async fn data(data: web::Data<AppState>) -> impl Responder {
    let csv = data.csv_data.lock().unwrap().latest_csv();
    HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, "text/csv"))
        .body(csv)
}

// POST /cmd
async fn cmd(
    data: web::Data<AppState>,
    payload: web::Json<CommandRequest>
) -> impl Responder {
    // Simulate command execution
    let mut csv_data = data.csv_data.lock().unwrap();
    let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs();

    match payload.command.as_str() {
        "set_temp" => {
            // Simulate setting temperature
            if let Some(params) = &payload.params {
                if let Some(temp) = params.get("temperature").and_then(|v| v.as_f64()) {
                    let row = vec![
                        now.to_string(),
                        format!("{:.2}", temp),
                        "ok".to_string()
                    ];
                    csv_data.rows.push_back(row);
                    if csv_data.rows.len() > 10 { csv_data.rows.pop_front(); }
                    return HttpResponse::Ok().json(serde_json::json!({"status": "success"}));
                }
            }
            HttpResponse::BadRequest().json(serde_json::json!({"status": "error", "message": "Missing temperature parameter"}))
        }
        _ => HttpResponse::BadRequest().json(serde_json::json!({"status": "error", "message": "Unknown command"})),
    }
}

// ====== Simulate Raw Protocol Fetch, Convert to HTTP CSV Stream ======
async fn stream_csv(data: web::Data<AppState>) -> Result<HttpResponse> {
    let csv_data = data.csv_data.lock().unwrap();
    let mut stream = csv_data.headers.join(",") + "\n";
    for row in csv_data.rows.iter() {
        stream += &row.join(",");
        stream += "\n";
    }
    Ok(HttpResponse::Ok()
        .insert_header((header::CONTENT_TYPE, "text/csv"))
        .insert_header((header::CACHE_CONTROL, "no-cache"))
        .body(stream))
}

// ========== Main ==========
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    // Read environment variables
    let server_host = get_env("SERVER_HOST", "0.0.0.0");
    let server_port = get_env("SERVER_PORT", "8080");

    // Simulated headers and data (can be replaced with real protocol fetch)
    let csv_headers = vec!["timestamp", "temperature", "status"];
    let mut csv_rows = VecDeque::new();
    csv_rows.push_back(vec![
        format!("{}", SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs()),
        "25.0".to_string(),
        "ok".to_string()
    ]);

    let device_info = DeviceInfo {
        device_name: "的v分·",
        device_model: "个人",
        manufacturer: "拰发·",
        device_type: " 为服务",
    };

    let csv_data = CsvData {
        headers: csv_headers,
        rows: csv_rows,
    };

    let state = web::Data::new(AppState {
        device_info,
        csv_data: Mutex::new(csv_data),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(Logger::default())
            .service(web::resource("/info").route(web::get().to(info)))
            .service(web::resource("/data").route(web::get().to(data)))
            .service(web::resource("/cmd").route(web::post().to(cmd)))
            .service(web::resource("/stream").route(web::get().to(stream_csv)))
    })
    .bind(format!("{}:{}", server_host, server_port))?
    .run()
    .await
}