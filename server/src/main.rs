use axum::body::Body;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use axum::http::{Response, StatusCode};
use axum::extract::Path;
use tokio;
use rusqlite::{Connection, Result};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "camelCase", serialize = "camelCase"))]
struct SetInput {
    machine_id: String,
    is_on: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct DeviceObjectSQL {
    device_id: String,
    display_name: String,
    last_updated: String,
    last_changed: String,
    is_on: bool
}

#[derive(Debug, Serialize, Deserialize)]
struct ErrorResponse {
    message: String,
    error: String
}

impl ErrorResponse {
    fn new(message: &str, error: rusqlite::Error) -> ErrorResponse {
        ErrorResponse { message: message.to_string(), error: format!("{error:?}") }
    }
}

#[derive(Debug)]
struct Database {
    connection: rusqlite::Connection
}

impl Database {
    fn open(filepath: &str) -> Result<Database> {
        let conn = Connection::open(filepath)?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS devices (
                device_id       TEXT NOT NULL PRIMARY KEY,
                display_name    TEXT NOT NULL,
                last_updated    TEXT NOT NULL,
                last_changed    TEXT NOT NULL,
                is_on           BOOLEAN
            )",
            (),
        )?;

        Ok(Database { connection: conn })
    }

    fn update_entry(&self, new_entry: SetInput) -> Result<DeviceObjectSQL> {
        // Get the corresponding entry from the database
        let mut statment = self.connection.prepare("SELECT * FROM devices WHERE device_id = ?1")?;
        let rows: Vec<DeviceObjectSQL> = statment.query_map([&new_entry.machine_id], |x| {
            Ok(DeviceObjectSQL {
                device_id: x.get(0)?,
                display_name: x.get(1)?,
                last_updated: x.get(2)?,
                last_changed: x.get(3)?,
                is_on: x.get(4)?
            })
        })?.filter_map(|x| x.ok()).collect();

        if rows.len() != 1 {
            return Err(rusqlite::Error::InvalidQuery)
        }

        let mut existing_entry = rows.get(0).ok_or(rusqlite::Error::InvalidQuery)?.clone();

        if existing_entry.is_on == new_entry.is_on {
            // No state change needed
            existing_entry.last_updated = chrono::Local::now().to_rfc3339();
        } else {
            // Change state
            let now = chrono::Local::now().to_rfc3339();
            existing_entry.last_changed = now.clone();
            existing_entry.last_updated = now;
            existing_entry.is_on = new_entry.is_on;

            // Change last changed and on state, for only the cases the state changes
            match self.connection.execute("UPDATE devices SET last_changed = ?1 WHERE device_id IS ?2;", (&existing_entry.last_changed, &existing_entry.device_id)) {
                Ok(_) => (),
                Err(e) => { return Err(e); }
            };

            match self.connection.execute("UPDATE devices SET is_on = ?1 WHERE device_id IS ?2;", (&existing_entry.is_on, &existing_entry.device_id)) {
                Ok(_) => (),
                Err(e) => { return Err(e); }
            };
        }

        // Now, change last update time for all cases
        match self.connection.execute("UPDATE devices SET last_updated = ?1 WHERE device_id IS ?2;", (&existing_entry.last_updated, &existing_entry.device_id)) {
            Ok(_) => Ok(existing_entry),
            Err(e) => Err(e),
        }
    }

    fn get_entry(&self, device_id: String) -> Result<DeviceObjectSQL> {
        // Get the corresponding entry from the database
        let mut statment = self.connection.prepare("SELECT * FROM devices WHERE device_id = ?1")?;
        let rows: Vec<DeviceObjectSQL> = statment.query_map([&device_id], |x| {
            Ok(DeviceObjectSQL {
                device_id: x.get(0)?,
                display_name: x.get(1)?,
                last_updated: x.get(2)?,
                last_changed: x.get(3)?,
                is_on: x.get(4)?
            })
        })?.filter_map(|x| x.ok()).collect();

        if rows.len() != 1 {
            return Err(rusqlite::Error::InvalidQuery)
        }

        let entry = rows.get(0).ok_or(rusqlite::Error::InvalidQuery)?.clone();

        Ok(entry)
    }
}

async fn set_data(Json(body): Json<SetInput>) -> Response<Body> {
    let db = match Database::open("data.db") {
        Ok(a) => a,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json::from(
                ErrorResponse::new("internal server error at read database", e))).into_response()
        }
    };

    match db.update_entry(body) {
        Ok(a) => (StatusCode::OK, Json::from(a)).into_response(),
        Err(e) => {
            println!("Err! {e:?}");
            (StatusCode::BAD_REQUEST, Json::from(
                ErrorResponse::new("bad request at update entry", e))).into_response()
        }
    }
}

async fn get_data(Path(device_id): Path<String>,) -> Result<Json<DeviceObjectSQL>, Response<Body>> {
    let db = match Database::open("data.db") {
        Ok(a) => a,
        Err(e) => { return Err((StatusCode::INTERNAL_SERVER_ERROR, Json::from(
            ErrorResponse::new("internal server error at read database", e))).into_response())
        }
    };

    match db.get_entry(device_id) {
        Ok(x) => Ok(Json::from(x)),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json::from(
            ErrorResponse::new("bad request at get entry", e))).into_response())
    }
}

async fn index() -> axum::response::Html<String> {
    axum::response::Html(std::fs::read_to_string("./pages/index.html").unwrap())
}

async fn index_css() -> axum::response::Response<String> {
    axum::response::Response::builder()
        .status(200)
        .header("Content-Type", "text/css")
        .body(std::fs::read_to_string("./pages/index.css").unwrap())
        .unwrap()
}

async fn index_js() -> axum::response::Response<String> {
    axum::response::Response::builder()
        .status(200)
        .header("Content-Type", "text/javascript")
        .body(std::fs::read_to_string("./pages/index.js").unwrap())
        .unwrap()
}


async fn setup_sqlite_db() {
    // Generates a new, empty database.
    let conn = Connection::open("data.db").expect("failed to open connection");

    conn.execute(
        "CREATE TABLE IF NOT EXISTS devices (
            device_id       TEXT NOT NULL PRIMARY KEY,
            display_name    TEXT NOT NULL,
            last_updated    TEXT NOT NULL,
            last_changed    TEXT NOT NULL,
            is_on           BOOLEAN
        )",
        (),
    ).expect("Failed to execute command");

    let now = chrono::Local::now().to_rfc3339();

    let plist = [
        DeviceObjectSQL {
            device_id: "THOMPSONS_FLOOR0_WASH".to_string(),
            display_name: "Thompsons Ground Floor Washing Machine".to_string(),
            is_on: false,
            last_updated: now.clone(),
            last_changed: now.clone()
        },
        DeviceObjectSQL {
            device_id: "THOMPSONS_FLOOR0_DRY".to_string(),
            display_name: "Thompsons Ground Floor Dryer".to_string(),
            is_on: false,
            last_updated: now.clone(),
            last_changed: now.clone()
        },
        DeviceObjectSQL {
            device_id: "THOMPSONS_FLOOR1_WASH".to_string(),
            display_name: "Thompsons First Floor Washing Machine".to_string(),
            is_on: false,
            last_updated: now.clone(),
            last_changed: now.clone()
        },
        DeviceObjectSQL {
            device_id: "THOMPSONS_FLOOR1_DRY".to_string(),
            display_name: "Thompsons First Floor Dryer".to_string(),
            is_on: false,
            last_updated: now.clone(),
            last_changed: now.clone()
        },
        DeviceObjectSQL {
            device_id: "THOMPSONS_FLOOR2_WASH".to_string(),
            display_name: "Thompsons Top Floor Washing Machine".to_string(),
            is_on: false,
            last_updated: now.clone(),
            last_changed: now.clone()
        },
        DeviceObjectSQL {
            device_id: "THOMPSONS_FLOOR2_DRY".to_string(),
            display_name: "Thompsons Top Floor Dryer".to_string(),
            is_on: false,
            last_updated: now.clone(),
            last_changed: now.clone()
        }
    ];

    for p in plist {
        conn.execute(
            "INSERT INTO devices VALUES (?1, ?2, ?3, ?4, ?5)",
            (&p.device_id, p.display_name, p.last_updated, p.last_changed, p.is_on),
        ).expect("failed to add");
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    let addr: &String = match args.get(1) {
        Some(a) => a,
        None => &String::from("localhost:3000")
    };

    let app = Router::new()
        .route("/get/{device_id}", get(get_data))
        .route("/set", post(set_data).get(index))
        .route("/", get(index))
        .route("/index.js", get(index_js))
        .route("/index.css", get(index_css));

    if !std::path::Path::new("data.db").exists() {
        setup_sqlite_db().await;
    }

    println!("Starting server on {addr}");

    axum::serve(
        tokio::net::TcpListener::bind(addr)
            .await
            .expect(&format!("ERROR: Could not bind to supplied address '{addr}'. Please check your permissions or select a different address. Details")[..]),
        app,
    )
    .await
    .unwrap();
}
