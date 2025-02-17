use std::net::{TcpListener, TcpStream};
use std::io::{BufRead, BufReader};
use rusqlite::{Connection, params};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // 1. Start listening on port 9000
    let listener = TcpListener::bind("0.0.0.0:9000")?;
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

    // 3. Accept incoming connections
    for stream in listener.incoming() {
        match stream {
            Ok(tcp_stream) => {
                println!("Client connected: {:?}", tcp_stream.peer_addr()?);
                handle_client(tcp_stream, &conn)?;
            }
            Err(e) => eprintln!("Connection failed: {}", e),
        }
    }

    Ok(())
}

fn handle_client(stream: TcpStream, conn: &Connection) -> Result<(), Box<dyn Error>> {
    let reader = BufReader::new(stream);

    // Process each line as one CSV record
    for line in reader.lines() {
        let line = line?;
        let line = line.trim();
        // Skip empty lines
        if line.is_empty() {
            continue;
        }
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

        // 4. Insert into the local sensor_data table
        conn.execute(
            "INSERT INTO sensor_data (
                sessionID, timestamp, latitude, longitude, altitude,
                accel_x, accel_y, accel_z, 
                gyro_x, gyro_y, gyro_z,
                dac_1, dac_2, dac_3, dac_4
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                session_id,
                timestamp,
                latitude,
                longitude,
                altitude,
                accel_x,
                accel_y,
                accel_z,
                gyro_x,
                gyro_y,
                gyro_z,
                dac_1,
                dac_2,
                dac_3,
                dac_4
            ],
        )?;
    }

    println!("Finished receiving data from client.");
    Ok(())
}