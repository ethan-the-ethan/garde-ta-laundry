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
use chrono;

const REVISION_NUMBER: u8 = 3;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all(deserialize = "camelCase", serialize = "camelCase"))]
struct SetInput {
    machine_id: String,
    is_on: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all(serialize = "camelCase"))]
struct DeviceObjectSQL {
    device_id: String,
    display_name: String,
    last_updated: String,
    last_changed: String,
    is_on: bool
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all(serialize = "camelCase"))]
struct DeviceHistoryWrapper {
    device: DeviceObjectSQL,
    history: Vec<(f64, bool)>
}

#[derive(Debug, Clone)]
struct HistoryEntry {
    time: chrono::DateTime<chrono::FixedOffset>,
    is_on: bool,
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

        conn.execute(
            "CREATE TABLE IF NOT EXISTS history (
                id              INTEGER PRIMARY KEY,
                device_id       TEXT NOT NULL,
                time            TEXT NOT NULL,
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
            existing_entry.last_updated = now.clone();
            existing_entry.is_on = new_entry.is_on;

            // Change last changed and on state, for only the cases the state changes
            match self.connection.execute(
                "UPDATE devices SET last_changed = ?1 WHERE device_id IS ?2;", (&existing_entry.last_changed, &existing_entry.device_id)) {
                Ok(_) => (),
                Err(e) => { return Err(e); }
            };

            match self.connection.execute(
                "INSERT INTO history VALUES (NULL, ?1, ?2, ?3)",
                (&existing_entry.device_id, now, existing_entry.is_on),
            ) {
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

    fn get_history(&self, device_id: &String) -> Result<Vec<(f64, bool)>> {
        // Get data from database
        let mut statment = self.connection.prepare("SELECT time, is_on FROM history WHERE device_id = ?1")?;
        let mut rows: Vec<HistoryEntry> = statment.query_map([&device_id], |x| {
            Ok((x.get(0)?, x.get(1)?))
        })?.filter_map(|x: std::prelude::v1::Result<(String, bool), rusqlite::Error>| match x {
            // Filter for only non-error values and convert the datetime strings to DateTime objects
            Ok(a) => {
                let time = chrono::DateTime::parse_from_rfc3339(&a.0[..]).unwrap();
                Some(HistoryEntry {
                    time: time,
                    is_on: a.1
                })
            },
            Err(_) => None
        }).collect();

        // Sort the rows, newest first
        rows.sort_by(|a, b| b.time.cmp(&a.time));

        let mut reached_end_flag = false;

        let now= chrono::Local::now().to_utc();
        let mut filtered_history: Vec<HistoryEntry> = Vec::new();

        // Take only the items in the past two hours, and the first item older than two hours (to get the state 2hrs ago)
        for item in rows {
            if reached_end_flag {
                break;
            } else if (now - item.time.to_utc()) > chrono::TimeDelta::seconds(7200) {
                reached_end_flag = true;
            }
            filtered_history.insert(0, item);
        }

        // Map each item to its duration and then a percentage of the total time period (2hrs)
        // This is for the history progress bar
        let m: Vec<(f64, bool)> = filtered_history.iter().clone().enumerate().map(|(i, entry)| {
            let now = chrono::Local::now().fixed_offset();

            let next_entry = match filtered_history.get(i + 1) {
                Some(a) => a.clone(),
                None => HistoryEntry { time: now, is_on: false }
            };

            // Here we take either two hours ago or the entry time, which ever is closer to present.
            // This is because the first item in the list will (almost) always be more than 2hrs ago.
            let duration = next_entry.time - entry.time.max(now.checked_sub_signed(chrono::TimeDelta::hours(2)).unwrap());

            ((duration.as_seconds_f64() / 72.0), entry.is_on)
        }).collect();

        Ok(m)
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

async fn get_data(Path(device_id): Path<String>) -> Result<Json<DeviceHistoryWrapper>, Response<Body>> {
    let db = match Database::open("data.db") {
        Ok(a) => a,
        Err(e) => { return Err((StatusCode::INTERNAL_SERVER_ERROR, Json::from(
            ErrorResponse::new("internal server error at read database", e))).into_response())
        }
    };

    let history = match db.get_history(&device_id) {
        Ok(x) => x,
        Err(e) => {
            return Err((StatusCode::BAD_REQUEST, Json::from(
                ErrorResponse::new("bad request at get history", e))).into_response())
        }
    };

    match db.get_entry(device_id) {
        Ok(x) => Ok(Json::from(DeviceHistoryWrapper {
            device: x,
            history
        })),
        Err(e) => Err((StatusCode::BAD_REQUEST, Json::from(
            ErrorResponse::new("bad request at get entry", e))).into_response())
    }
}

async fn get_history(Path(device_id): Path<String>) -> Result<Json<Vec<(f64, bool)>>, Response<Body>> {
    let db = match Database::open("data.db") {
        Ok(a) => a,
        Err(e) => { return Err((StatusCode::INTERNAL_SERVER_ERROR, Json::from(
            ErrorResponse::new("internal server error at read database", e))).into_response())
        }
    };

    // Get last two hours of history
    let history = match db.get_history(&device_id) {
        Ok(a) => a,
        Err(e) => { return Err((StatusCode::INTERNAL_SERVER_ERROR, Json::from(
            ErrorResponse::new("internal server error at get history", e))).into_response())
        }
    };

    Ok(Json::from(history))
}

async fn index() -> axum::response::Html<String> {
    axum::response::Html(std::fs::read_to_string("./pages/index.html").unwrap())
}

async fn about() -> axum::response::Html<String> {
    axum::response::Html(std::fs::read_to_string("./pages/about.html").unwrap())
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

    conn.execute(
        "CREATE TABLE IF NOT EXISTS history (
            id              INTEGER PRIMARY KEY,
            device_id       TEXT NOT NULL,
            time            TEXT NOT NULL,
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

        conn.execute(
            "INSERT INTO history VALUES (NULL, ?1, ?2, ?3)",
            (&p.device_id, now.clone(), p.is_on),
        ).expect("failed to add");
    }
}

async fn create_test_db() {
    // Creates a test database with some placeholder times.
    let db = Database::open("data.db").unwrap();

    let now = chrono::Local::now().fixed_offset();
    let four_mins_ago = now.checked_sub_signed(chrono::TimeDelta::minutes(4)).unwrap();
    let half_hour_ago = now.checked_sub_signed(chrono::TimeDelta::minutes(30)).unwrap();
    let two_hours_ago = now.checked_sub_signed(chrono::TimeDelta::minutes(120)).unwrap();
    let fifteen_mins_ago = now.checked_sub_signed(chrono::TimeDelta::minutes(15)).unwrap();

    db.connection.execute(
        "INSERT INTO history VALUES (NULL, ?1, ?2, ?3)",
        ("THOMPSONS_FLOOR0_WASH", four_mins_ago.to_rfc3339(), true),
    ).expect("failed to add");

    db.connection.execute(
        "INSERT INTO history VALUES (NULL, ?1, ?2, ?3)",
        ("THOMPSONS_FLOOR0_WASH", fifteen_mins_ago.to_rfc3339(), false),
    ).expect("failed to add");

    db.connection.execute(
        "INSERT INTO history VALUES (NULL, ?1, ?2, ?3)",
        ("THOMPSONS_FLOOR0_WASH", half_hour_ago.to_rfc3339(), true),
    ).expect("failed to add");

    db.connection.execute(
        "INSERT INTO history VALUES (NULL, ?1, ?2, ?3)",
        ("THOMPSONS_FLOOR0_WASH", two_hours_ago.to_rfc3339(), false),
    ).expect("failed to add");

    db.connection.execute(
        "INSERT INTO history VALUES (NULL, ?1, ?2, ?3)",
        ("THOMPSONS_FLOOR0_DRY", two_hours_ago.to_rfc3339(), true),
    ).expect("failed to add");

    db.connection.execute(
        "INSERT INTO history VALUES (NULL, ?1, ?2, ?3)",
        ("THOMPSONS_FLOOR0_DRY", fifteen_mins_ago.to_rfc3339(), false),
    ).expect("failed to add");

    for machine in [
        "THOMPSONS_FLOOR1_DRY",
        "THOMPSONS_FLOOR1_WASH",
        "THOMPSONS_FLOOR2_DRY",
        "THOMPSONS_FLOOR2_WASH"
    ] {
        db.connection.execute(
            "INSERT INTO history VALUES (NULL, ?1, ?2, ?3)",
            (machine, two_hours_ago.to_rfc3339(), false),
        ).expect("failed to add");
    }

    let _ = db.update_entry(SetInput { machine_id: "THOMPSONS_FLOOR0_WASH".to_string(), is_on: true });
    let _ = db.update_entry(SetInput { machine_id: "THOMPSONS_FLOOR0_DRY".to_string(), is_on: false });
}

#[tokio::main]
async fn main() {
    println!("gtl-server revision {REVISION_NUMBER}");

    let args: Vec<String> = env::args().collect();
    let addr: &String = match args.get(1) {
        Some(a) => a,
        None => &String::from("localhost:3000")
    };

    let app = Router::new()
        .route("/get/{device_id}", get(get_data))
        .route("/history/{device_id}", get(get_history))
        .route("/set", post(set_data).get(index))
        .route("/", get(index))
        .route("/index.js", get(index_js))
        .route("/index.css", get(index_css))
        .route("/about", get(about));

    if !std::path::Path::new("data.db").exists() {
        setup_sqlite_db().await;
        create_test_db().await;
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
