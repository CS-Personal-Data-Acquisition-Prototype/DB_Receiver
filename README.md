# DB_Receiver

A TCP server application that receives sensor data from remote clients (like Raspberry Pi) and stores it in a local SQLite database.

## Overview

This application acts as a data collection endpoint for IoT or sensor systems. It:
- Listens for TCP connections on port 9000
- Receives CSV-formatted sensor data from connected clients
- Parses the data and stores it in a SQLite database
- Handles multiple concurrent client connections
- Provides graceful shutdown with Ctrl+C

## Testing Environment

This application is specifically designed for testing data transfer from Raspberry Pi devices to a local machine. It serves as a reliable endpoint to verify that sensor data can be successfully transmitted over a network connection and stored persistently.

## Prerequisites

- Rust (latest stable version recommended)
- Cargo package manager
- SQLite

## Dependencies

- `rusqlite`: SQLite database interaction
- `ctrlc`: Signal handling for graceful shutdown

## Installation

1. Clone the repository:
   ```
   git clone <repository-url>
   cd DB_Receiver
   ```

2. Build the application:
   ```
   cargo build --release
   ```

## Running the Server

Start the server application:

```
cargo run --release
```

The server will:
- Listen on 0.0.0.0:9000 (all network interfaces)
- Create a SQLite database file named `received_data.db` if it doesn't exist
- Print connection information to the console

To stop the server, press `Ctrl+C` for a graceful shutdown.

## Database Structure

The application creates a `sensor_data` table with the following schema:

| Column    | Type    | Description                          |
|-----------|---------|--------------------------------------|
| id        | INTEGER | Primary key (auto-incremented)       |
| sessionID | INTEGER | Session identifier                   |
| timestamp | TEXT    | Data collection timestamp            |
| latitude  | REAL    | GPS latitude                         |
| longitude | REAL    | GPS longitude                        |
| altitude  | REAL    | GPS altitude                         |
| accel_x   | REAL    | Accelerometer X-axis reading         |
| accel_y   | REAL    | Accelerometer Y-axis reading         |
| accel_z   | REAL    | Accelerometer Z-axis reading         |
| gyro_x    | REAL    | Gyroscope X-axis reading             |
| gyro_y    | REAL    | Gyroscope Y-axis reading             |
| gyro_z    | REAL    | Gyroscope Z-axis reading             |
| dac_1     | REAL    | Data acquisition channel 1           |
| dac_2     | REAL    | Data acquisition channel 2           |
| dac_3     | REAL    | Data acquisition channel 3           |
| dac_4     | REAL    | Data acquisition channel 4           |

## Connection Details

- **Protocol**: TCP
- **Port**: 9000
- **Data Format**: CSV with 15 fields in the following order:
  1. Session ID (integer or "None")
  2. Timestamp (string)
  3. Latitude (float)
  4. Longitude (float)
  5. Altitude (float)
  6. Accelerometer X (float)
  7. Accelerometer Y (float)
  8. Accelerometer Z (float)
  9. Gyroscope X (float)
  10. Gyroscope Y (float)
  11. Gyroscope Z (float)
  12. DAC Channel 1 (float)
  13. DAC Channel 2 (float)
  14. DAC Channel 3 (float)
  15. DAC Channel 4 (float)

## Testing with Raspberry Pi

To test data transfer from a Raspberry Pi:

1. Ensure the Pi and the machine running this server are on the same network
2. Configure your Pi application to send data to the server's IP address on port 9000
3. Format the data as CSV according to the specification above
4. Each line sent should contain one complete data record

Example Python code for the Raspberry Pi client:

```python
import socket
import time

# Replace with your server's IP address
SERVER_IP = '192.168.1.100'  
SERVER_PORT = 9000

# Sample data
session_id = 1
timestamp = "2023-01-01T12:00:00"
# Add your sensor readings here

# Create a TCP connection
with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
    s.connect((SERVER_IP, SERVER_PORT))
    
    # Format data as CSV and send
    data = f"{session_id},{timestamp},0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0\n"
    s.sendall(data.encode())
    
    # Wait to ensure data is sent
    time.sleep(1)

print("Data sent successfully")
```

## Viewing Collected Data

You can use any SQLite client to view the collected data:

```
sqlite3 received_data.db "SELECT * FROM sensor_data;"
```

## Performance Considerations

- The server is designed to handle multiple concurrent connections
- Each client connection is processed in its own thread
- The database is shared among all connections
- Connection timeout is set to 5 minutes of inactivity

## License Notice
To apply the Apache License to your work, attach the following boilerplate notice. The text should be enclosed in the appropriate comment syntax for the file format. We also recommend that a file or class name and description of purpose be included on the same "printed page" as the copyright notice for easier identification within third-party archives.

    Copyright 2025 CS 462 Personal Data Acquisition Prototype Group
    
    Licensed under the Apache License, Version 2.0 (the "License"); you may not use this file except in compliance with the License. You may obtain a copy of the License at
    
    http://www.apache.org/licenses/LICENSE-2.0
    Unless required by applicable law or agreed to in writing, software distributed under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the License for the specific language governing permissions and limitations under the License.
