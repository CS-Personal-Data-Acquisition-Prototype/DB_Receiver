use std::net::{TcpListener, TcpStream, Shutdown};
use std::io::{self, BufRead, BufReader, ErrorKind};
use rusqlite::{Connection, params};
use std::error::Error;
use std::thread;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use ctrlc;
use serde::{Deserialize, Serialize};
use serde_json;
use std::time::SystemTime;

// Define struct to match the expected JSON structure
#[derive(Serialize, Deserialize, Debug)]
struct SensorData {
    sessionID: Option<i32>,
    timestamp: String,
    latitude: f64,
    longitude: f64,
    altitude: f64,
    accel_x: f64,
    accel_y: f64,
    accel_z: f64,
    gyro_x: f64,
    gyro_y: f64,
    gyro_z: f64,
    dac_1: f64,
    dac_2: f64,
    dac_3: f64,
    dac_4: f64,
}

// Struct for keepalive messages
#[derive(Serialize, Deserialize, Debug)]
struct KeepaliveMessage {
    #[serde(rename = "type")]
    message_type: String,
}

// Enum to handle different message types
#[derive(Debug)]
enum Message {
    SensorData(SensorData),
    Keepalive,
    Unknown,
}

fn main() -> Result<(), Box<dyn Error>> {
    // 1. Start listening on port 9000
    let listener = TcpListener::bind("0.0.0.0:9000")?;
    listener.set_nonblocking(true)?;
    println!("Server listening on port 9000...");
    
    // 2. Open or create a local database
    let conn = Connection::open("received_data.db")?;
    
    // Create table if it doesn't exist
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sensor_data (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            sessionID INTEGER,
            timestamp TEXT,
            latitude REAL,
            longitude REAL,
            altitude REAL,
            accel_x REAL,
            accel_y REAL,
            accel_z REAL,
            gyro_x REAL,
            gyro_y REAL,
            gyro_z REAL,
            dac_1 REAL,
            dac_2 REAL,
            dac_3 REAL,
            dac_4 REAL
        )",
        [],
    )?;

    // Create a shared flag for graceful shutdown
    let running = Arc::new(Mutex::new(true));
    let r = running.clone();
    
    // Set up ctrl-c handler for graceful shutdown
    ctrlc::set_handler(move || {
        println!("Shutdown signal received, closing server gracefully...");
        let mut running = r.lock().unwrap();
        *running = false;
    })?;

    // Track client threads
    let mut client_threads = Vec::new();

    // 3. Accept incoming connections
    while *running.lock().unwrap() {
        match listener.accept() {
            Ok((stream, addr)) => {
                println!("Client connected: {:?}", addr);
                
                // Make the client stream blocking for reliable data transfer
                stream.set_nonblocking(false).unwrap_or_else(|e| {
                    eprintln!("Warning: Could not set client socket to blocking mode: {}", e);
                });
                
                // Open a new database connection for this thread
                let thread_conn = match Connection::open("received_data.db") {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Failed to open database connection: {}", e);
                        continue;
                    }
                };
                
                // Handle each client in a separate thread
                let handle = thread::spawn(move || {
                    if let Err(e) = handle_client(stream, &thread_conn) {
                        eprintln!("Error handling client {}: {}", addr, e);
                    }
                    println!("Connection from {} ended", addr);
                });
                
                client_threads.push(handle);
                
                // Clean up completed threads
                client_threads.retain(|h| !h.is_finished());
            }
            Err(e) => {
                if e.kind() == io::ErrorKind::WouldBlock {
                    // No connection available, sleep briefly and check running flag
                    thread::sleep(Duration::from_millis(100));
                } else {
                    eprintln!("Connection error: {}", e);
                    thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }

    println!("Server shutting down... waiting for client connections to finish");
    
    // Wait for active client threads to complete (optional timeout could be added)
    for handle in client_threads {
        let _ = handle.join();
    }

    println!("Server shutdown complete");
    Ok(())
}

fn handle_client(mut stream: TcpStream, conn: &Connection) -> Result<(), Box<dyn Error>> {
    // Set read timeout instead of using non-blocking mode
    stream.set_read_timeout(Some(Duration::from_secs(300)))?; // 5 minutes
    
    // Use larger buffer size
    let reader = BufReader::with_capacity(8192, stream);

    // Process each line as one JSON record
    for line in reader.lines() {
        match line {
            Ok(line) => {
                let line = line.trim();
                // Skip empty lines
                if line.is_empty() {
                    continue;
                }
                
                // Debug output to see what's being received
                println!("Received data: {}", line);
                
                // First check if the line contains "keepalive" before attempting to parse
                if line.contains("\"type\":\"keepalive\"") {
                    println!("Received keepalive message");
                    continue; // Skip further processing for this line
                }
                
                // Try to parse as sensor data
                match serde_json::from_str::<SensorData>(&line) {
                        Ok(data) => {
                            // Additional validation - skip if timestamp is "keepalive"
                            if data.timestamp == "keepalive" || data.timestamp.contains("keepalive") {
                                println!("Detected keepalive disguised as sensor data");
                                continue;
                            }
                                                        
                            // Insert into the database
                            if let Err(e) = conn.execute(
                                "INSERT INTO sensor_data (
                                    sessionID, timestamp, latitude, longitude, altitude,
                                    accel_x, accel_y, accel_z, 
                                    gyro_x, gyro_y, gyro_z,
                                    dac_1, dac_2, dac_3, dac_4
                                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                                params![
                                    data.sessionID, data.timestamp, data.latitude, data.longitude, data.altitude,
                                    data.accel_x, data.accel_y, data.accel_z, 
                                    data.gyro_x, data.gyro_y, data.gyro_z,
                                    data.dac_1, data.dac_2, data.dac_3, data.dac_4
                                ],
                            ) {
                                eprintln!("Database error: {}", e);
                            } else {
                                println!("Data successfully inserted into database");
                            }
                        },
                    Err(e) => {
                        eprintln!("JSON parsing error: {}", e);
                        eprintln!("Invalid JSON data: {}", line);
                    }
                }
            },
            Err(e) => {
                // Handle connection errors
                if e.kind() == ErrorKind::TimedOut {
                    continue; // Just a timeout, keep waiting
                } else if e.kind() == ErrorKind::WouldBlock {
                    // No data available right now, wait briefly
                    thread::sleep(Duration::from_millis(10));
                    continue;
                } else {
                    // Client disconnected or other error
                    println!("Client disconnected: {}", e);
                    break;
                }
            }
        }
    }

    println!("Finished receiving data from client.");
    Ok(())
}