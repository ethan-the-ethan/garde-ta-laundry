use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use axum::http::StatusCode;
use axum::extract::Path;
use tokio;
use rusqlite::{Connection, Result};

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
            (), // empty list of parameters.
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
            existing_entry.last_updated = chrono::Local::now().to_string();
        } else {
            // Change state
            let now = chrono::Local::now().to_string();
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

async fn set_data(Json(body): Json<SetInput>) -> StatusCode {
    let db = match Database::open("data.db") {
        Ok(a) => a,
        Err(_) => { return StatusCode::INTERNAL_SERVER_ERROR }
    };

    match db.update_entry(body) {
        Ok(_) => StatusCode::OK,
        Err(e) => {
            println!("Err! {e:?}");
            StatusCode::BAD_REQUEST
        }
    }
}

async fn get_data(Path(device_id): Path<String>,) -> Result<Json<DeviceObjectSQL>, StatusCode> {
    let db = match Database::open("data.db") {
        Ok(a) => a,
        Err(_) => { return Err(StatusCode::INTERNAL_SERVER_ERROR) }
    };

    let fetched_entry = db.get_entry(device_id);

    if !fetched_entry.is_ok() { return Err(StatusCode::BAD_REQUEST); }

    Ok(Json::from(fetched_entry.unwrap()))
}

async fn index() -> axum::response::Html<String> {
    axum::response::Html(std::fs::read_to_string("./pages/index.html").unwrap())
}

async fn setup_sqlite_db() {
    // This is a one and done thing. Generates a new database.
    let conn = Connection::open("data.db").expect("failed to open connection");

    conn.execute(
            "CREATE TABLE IF NOT EXISTS devices (
                device_id       TEXT NOT NULL PRIMARY KEY,
                display_name    TEXT NOT NULL,
                last_updated    TEXT NOT NULL,
                last_changed    TEXT NOT NULL,
                is_on           BOOLEAN
            )",
            (), // empty list of parameters.
        ).expect("Failed to execute command");

    let plist = [DeviceObjectSQL {
        device_id: "THOMPSONS_FLOOR0_WASH".to_string(),
        display_name: "Thompsons Ground Floor Dryer".to_string(),
        is_on: false,
        last_updated: chrono::Local::now().to_string(),
        last_changed: chrono::Local::now().to_string()
    }, DeviceObjectSQL {
        device_id: "THOMPSONS_FLOOR0_DRY".to_string(),
        display_name: "Thompsons Ground Floor Dryer".to_string(),
        is_on: false,
        last_updated: chrono::Local::now().to_string(),
        last_changed: chrono::Local::now().to_string()
    },DeviceObjectSQL {
        device_id: "THOMPSONS_FLOOR1_WASH".to_string(),
        display_name: "Thompsons First Floor Dryer".to_string(),
        is_on: false,
        last_updated: chrono::Local::now().to_string(),
        last_changed: chrono::Local::now().to_string()
    }, DeviceObjectSQL {
        device_id: "THOMPSONS_FLOOR1_DRY".to_string(),
        display_name: "Thompsons First Floor Dryer".to_string(),
        is_on: false,
        last_updated: chrono::Local::now().to_string(),
        last_changed: chrono::Local::now().to_string()
    }, DeviceObjectSQL {
        device_id: "THOMPSONS_FLOOR2_WASH".to_string(),
        display_name: "Thompsons Top Floor Dryer".to_string(),
        is_on: false,
        last_updated: chrono::Local::now().to_string(),
        last_changed: chrono::Local::now().to_string()
    },
    DeviceObjectSQL {
        device_id: "THOMPSONS_FLOOR2_DRY".to_string(),
        display_name: "Thompsons Top Floor Dryer".to_string(),
        is_on: false,
        last_updated: chrono::Local::now().to_string(),
        last_changed: chrono::Local::now().to_string()
    }];

    for p in plist {
        conn.execute(
            "INSERT INTO devices VALUES (?1, ?2, ?3, ?4, ?5)",
            (&p.device_id, p.display_name, p.last_updated, p.last_changed, p.is_on),
        ).expect("failed to add");
    }
}

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/get/{device_id}", get(get_data))
        .route("/set", post(set_data).get(index));

    if !std::path::Path::new("data.db").exists() {
        setup_sqlite_db().await;
    }

    axum::serve(
        tokio::net::TcpListener::bind("localhost:3000")
            .await
            .unwrap(),
        app,
    )
    .await
    .unwrap();
}
