use std::net::{TcpListener, TcpStream, Shutdown};
use std::io::{self, BufRead, BufReader, ErrorKind};
use rusqlite::{Connection, params};
use std::error::Error;
use std::thread;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use ctrlc;

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

    // Process each line as one CSV record
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
                
                // Expect 15 comma-separated fields
                let fields: Vec<&str> = line.split(',').collect();
                if fields.len() < 15 {
                    eprintln!("Received incomplete data: {}", line);
                    continue;
                }

                // Parse each field
                let session_id = if fields[0] == "None" {
                    None
                } else {
                    fields[0].parse::<i32>().ok()
                };
                let timestamp = fields[1];
                let latitude = fields[2].parse::<f64>().unwrap_or(0.0);
                let longitude = fields[3].parse::<f64>().unwrap_or(0.0);
                let altitude = fields[4].parse::<f64>().unwrap_or(0.0);
                let accel_x = fields[5].parse::<f64>().unwrap_or(0.0);
                let accel_y = fields[6].parse::<f64>().unwrap_or(0.0);
                let accel_z = fields[7].parse::<f64>().unwrap_or(0.0);
                let gyro_x = fields[8].parse::<f64>().unwrap_or(0.0);
                let gyro_y = fields[9].parse::<f64>().unwrap_or(0.0);
                let gyro_z = fields[10].parse::<f64>().unwrap_or(0.0);
                let dac_1 = fields[11].parse::<f64>().unwrap_or(0.0);
                let dac_2 = fields[12].parse::<f64>().unwrap_or(0.0);
                let dac_3 = fields[13].parse::<f64>().unwrap_or(0.0);
                let dac_4 = fields[14].parse::<f64>().unwrap_or(0.0);

                // Insert into the database, with error handling
                if let Err(e) = conn.execute(
                    "INSERT INTO sensor_data (
                        sessionID, timestamp, latitude, longitude, altitude,
                        accel_x, accel_y, accel_z, 
                        gyro_x, gyro_y, gyro_z,
                        dac_1, dac_2, dac_3, dac_4
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    params![
                        session_id, timestamp, latitude, longitude, altitude,
                        accel_x, accel_y, accel_z, 
                        gyro_x, gyro_y, gyro_z,
                        dac_1, dac_2, dac_3, dac_4
                    ],
                ) {
                    eprintln!("Database error: {}", e);
                    // Continue processing instead of returning error
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