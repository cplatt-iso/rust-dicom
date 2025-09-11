# DICOM Rust Project - Sender and Receiver

This project has been reorganized to support both DICOM C-STORE sending and receiving functionality.

## Project Structure

```
src/
├── common/           # Shared code between sender and receiver
│   ├── types.rs      # Common data types (DicomFile, TransferStats, etc.)
│   ├── sop_classes.rs      # SOP Class definitions and registries
│   ├── transfer_syntaxes.rs # Transfer syntax definitions and registries
│   └── mod.rs        # Module exports
├── sender/          # DICOM C-STORE sender implementation
│   ├── main.rs      # Sender binary entry point
│   ├── dicom_client.rs # Core sending logic
│   └── mod.rs       # Module exports
├── receiver/        # DICOM C-STORE receiver implementation
│   ├── main.rs      # Receiver binary entry point
│   ├── receiver.rs  # Core receiving logic
│   └── mod.rs       # Module exports
├── bin/             # Utility binaries
│   ├── show_sop_classes.rs
│   └── show_transfer_syntaxes.rs
└── main.rs          # Project information entry point
```

## Binaries

The project builds two main binaries:

### DICOM Sender (`dicom-sender`)

High-performance async DICOM C-STORE sender that supports:
- Multiple concurrent associations/threads
- Comprehensive SOP class and transfer syntax support
- Progress tracking and detailed logging
- Recursive directory scanning
- Session summaries with performance metrics

Usage:
```bash
cargo run --bin dicom-sender -- --input /path/to/dicom/files --ae-title TARGET_AE --host 192.168.1.100 --port 4242 --threads 4 --recursive
```

### DICOM Receiver (`dicom-receiver`)

Async DICOM C-STORE receiver that supports:
- Multiple concurrent associations
- Same SOP class and transfer syntax support as sender
- Configurable output directory
- Association negotiation and validation
- Background processing of received files

Usage:
```bash
cargo run --bin dicom-receiver -- --output /path/to/output --port 4242 --ae-title MY_SCP --max-connections 10
```

## Features

### Shared Functionality
- **SOP Classes**: Comprehensive support for 180+ DICOM SOP classes across all major modalities
- **Transfer Syntaxes**: Support for uncompressed, compressed (JPEG, JPEG 2000, RLE), and specialized transfer syntaxes
- **Async Architecture**: Built with Tokio for high performance concurrent operations
- **Logging**: Detailed logging with tracing and session management

### Sender Features
- Multi-threaded sending with configurable concurrency
- Progress bars and real-time statistics
- Study-based grouping and batch processing
- JSON summary reports
- Error handling and retry logic

### Receiver Features
- Multi-connection support with semaphore-based limiting
- DICOM association negotiation
- Presentation context evaluation
- Automatic file saving with timestamp naming
- Graceful connection handling and cleanup

## Building

```bash
# Build both binaries
cargo build --release

# Build just the sender
cargo build --bin dicom-sender --release

# Build just the receiver  
cargo build --bin dicom-receiver --release
```

## Example Usage

Start a receiver:
```bash
./target/release/dicom-receiver --output /tmp/received --port 4242 --ae-title TEST_SCP
```

Send files to the receiver:
```bash
./target/release/dicom-sender --input test-dicom/ --recursive --ae-title TEST_SCP --host localhost --port 4242 --threads 2
```

## Configuration

Both binaries support various configuration options:

**Sender:**
- Input path (file or directory)
- Recursive scanning
- Target AE title, host, and port
- Number of concurrent threads
- Calling AE title
- Verbose logging

**Receiver:**
- Output directory for received files
- Listen port
- AE title for the receiver
- Maximum concurrent connections
- Verbose logging

## Development

The project uses a modular architecture with shared code in `src/common/`. This allows for:
- Code reuse between sender and receiver
- Consistent SOP class and transfer syntax handling
- Shared data types and utilities
- Independent binary compilation

## Logs

Both binaries create detailed logs in the `logs/` directory with unique session IDs. The sender also creates JSON summary files with transfer statistics.
