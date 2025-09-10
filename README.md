# DICOM C-STORE Sender

A high-performance, multi-threaded DICOM C-STORE client implementation written in Rust. This application provides reliable DICOM file transmission capabilities with support for concurrent transfers, proper DICOM protocol compliance, and comprehensive logging.

## Overview

This tool implements the DICOM C-STORE operation for sending DICOM files to PACS (Picture Archiving and Communication System) servers or other DICOM Storage Service Class Providers. It features intelligent study-based organization, concurrent processing, and robust error handling.

### Key Features

- **DICOM Protocol Compliance**: Full implementation of DICOM Upper Layer Protocol and C-STORE operations
- **Multi-threaded Processing**: Configurable concurrent transfers for improved performance
- **Study-based Organization**: Automatically groups files by Study Instance UID for logical transfer batches
- **Transfer Syntax Negotiation**: Supports Explicit VR Little Endian and Implicit VR Little Endian
- **PDU Fragmentation**: Handles large DICOM files with proper PDU segmentation
- **Comprehensive Logging**: Detailed session logs and JSON summaries for analysis
- **Progress Monitoring**: Real-time progress indicators and transfer statistics
- **Flexible Input**: Supports single files, directories, and recursive directory scanning

## Technology Stack

- **Language**: Rust (2021 edition)
- **DICOM Library**: dicom-rs 0.8.x ecosystem
  - `dicom-core`: Core DICOM data structures and parsing
  - `dicom-ul`: DICOM Upper Layer Protocol implementation
  - `dicom-object`: DICOM object manipulation
  - `dicom-transfer-syntax-registry`: Transfer syntax support
- **Networking**: TCP-based DICOM associations with proper state management
- **Concurrency**: Tokio async runtime with multi-threaded execution
- **CLI Framework**: Clap for command-line argument parsing
- **Logging**: Tracing framework with structured logging
- **Progress Display**: Indicatif for console progress bars

## System Requirements

### Runtime Requirements
- **Operating System**: Linux, macOS, or Windows
- **Network**: TCP connectivity to target DICOM server
- **Memory**: Minimum 512MB RAM (more recommended for large datasets)
- **Disk**: Sufficient space for log files (typically <1MB per session)

### Build Requirements
- **Rust**: Version 1.70.0 or later
- **Cargo**: Included with Rust installation
- **Git**: For cloning the repository

## Installation

### Download and Build from Source

```bash
# Clone the repository
git clone https://github.com/cplatt-iso/rust-dicom.git
cd rust-dicom

# Build in release mode for optimal performance
cargo build --release

# The binary will be available at ./target/release/dicom-sender
```

### Verify Installation

```bash
# Check that the application runs and displays help
./target/release/dicom-sender --help
```

## Usage

### Basic Syntax

```bash
dicom-sender --input <PATH> --ae-title <AE_TITLE> --host <HOST> --port <PORT> [OPTIONS]
```

### Command Line Options

```
Required Arguments:
  -i, --input <INPUT>              Input path (file or directory)
  -a, --ae-title <AE_TITLE>        Called AE Title (destination server)
  -H, --host <HOST>                Destination IP address or hostname
  -p, --port <PORT>                Destination port number

Optional Arguments:
  -r, --recursive                  Enable recursive directory scanning
  -c, --calling-ae <CALLING_AE>    Calling AE Title [default: RUST_SCU]
  -t, --threads <THREADS>          Number of concurrent threads [default: 1]
  -v, --verbose                    Enable verbose console output
  -h, --help                       Display help information
  -V, --version                    Display version information
```

### Usage Examples

#### Send a Single DICOM File
```bash
./target/release/dicom-sender \
  --input /path/to/image.dcm \
  --ae-title PACS_SERVER \
  --host 192.168.1.100 \
  --port 4242
```

#### Send All Files in a Directory
```bash
./target/release/dicom-sender \
  --input /data/dicom_files \
  --recursive \
  --ae-title ARCHIVE_SCP \
  --host pacs.hospital.local \
  --port 11112
```

#### High-Performance Multi-threaded Transfer
```bash
./target/release/dicom-sender \
  --input /mnt/studies \
  --recursive \
  --threads 8 \
  --calling-ae WORKSTATION_01 \
  --ae-title CENTRAL_PACS \
  --host 10.0.50.100 \
  --port 4242 \
  --verbose
```

## Architecture

### DICOM Protocol Implementation

The application implements the following DICOM protocol components:

- **Association Establishment**: Proper A-ASSOCIATE-RQ/AC negotiation
- **Presentation Contexts**: Support for multiple SOP Classes and Transfer Syntaxes
- **C-STORE Operations**: Complete C-STORE-RQ/RSP implementation with dataset transfer
- **PDU Management**: Fragmentation and reassembly for large datasets
- **Association Release**: Clean A-RELEASE-RQ/RP termination

### Supported SOP Classes

- Secondary Capture Image Storage (1.2.840.10008.5.1.4.1.1.7)
- CT Image Storage (1.2.840.10008.5.1.4.1.1.2)
- MR Image Storage (1.2.840.10008.5.1.4.1.1.4)
- Computed Radiography Image Storage (1.2.840.10008.5.1.4.1.1.1)

### Transfer Syntaxes

- Explicit VR Little Endian (1.2.840.10008.1.2.1)
- Implicit VR Little Endian (1.2.840.10008.1.2)

### Processing Pipeline

1. **File Discovery**: Scan input paths and identify DICOM files
2. **Header Parsing**: Extract Study Instance UID and SOP metadata
3. **Study Grouping**: Organize files by Study Instance UID
4. **Thread Distribution**: Assign studies to worker threads
5. **DICOM Transfer**: Establish associations and perform C-STORE operations
6. **Result Aggregation**: Collect statistics and generate reports

### Multi-threading Strategy

Files are grouped by Study Instance UID and distributed across worker threads:

```
Thread 1: Study A (30 files) → Study D (15 files)
Thread 2: Study B (25 files) → Study E (20 files)
Thread 3: Study C (40 files) → Study F (10 files)
```

This approach ensures:
- Logical grouping of related files
- Balanced workload distribution
- Minimal association overhead
- Optimal network utilization

## Output and Logging

### Console Output

The application provides real-time feedback including:
- Session identification and log file locations
- File discovery and study grouping statistics
- Progress bars showing transfer completion
- Final summary with performance metrics

### Log Files

All session data is written to the `logs/` directory:

#### Detailed Log (`logs/dicom_sender_<session_id>.log`)
Structured text log with:
- Timestamp and log level for each event
- Association establishment and termination
- Individual file transfer results
- Error messages and diagnostic information

#### JSON Summary (`logs/dicom_sender_summary_<session_id>.json`)
Machine-readable summary including:
- Session metadata and timing information
- Transfer statistics (success/failure counts, throughput)
- Performance metrics (average transfer time, total bytes)
- Configuration parameters used

## Performance Considerations

### Thread Configuration

- **1 thread**: Suitable for testing and small datasets
- **2-4 threads**: Optimal for most production scenarios
- **8+ threads**: May overwhelm destination servers or network infrastructure

### Network Optimization

- Monitor network bandwidth utilization during transfers
- Consider destination server processing capacity
- Adjust thread count based on file sizes and network latency
- Ensure adequate network buffer sizes for optimal TCP performance

### File Organization

Best performance is achieved when:
- DICOM files have valid, complete headers
- Files are organized logically on disk (e.g., by study)
- Network connectivity is stable with low latency
- Destination server has sufficient processing capacity

## Dependencies

Core dependencies managed by Cargo:

```toml
[dependencies]
dicom = "0.8"
dicom-ul = "0.8"
tokio = { version = "1.0", features = ["full"] }
clap = { version = "4.0", features = ["derive"] }
tracing = "0.1"
tracing-subscriber = "0.3"
indicatif = "0.17"
console = "0.15"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
uuid = { version = "1.0", features = ["v4"] }
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1.0"
smallvec = "1.10"
```

## Error Handling

The application provides comprehensive error reporting for:

- **File System Errors**: Invalid paths, permission issues, corrupted files
- **DICOM Protocol Errors**: Association failures, invalid SOP Classes, transfer syntax mismatches
- **Network Errors**: Connection timeouts, server rejections, transmission failures
- **Parse Errors**: Invalid DICOM headers, unsupported transfer syntaxes

All errors are logged with context information to assist with troubleshooting.

## Contributing

Contributions are welcome! Please ensure:

- Code follows Rust conventions and passes `cargo fmt` and `cargo clippy`
- New features include appropriate tests
- Documentation is updated for API changes
- Commit messages clearly describe changes

## License

This project is licensed under the MIT License. See the LICENSE file for details.

## Support

For issues, feature requests, or questions:

1. Check existing GitHub issues
2. Create a new issue with detailed information
3. Include relevant log files and configuration details
