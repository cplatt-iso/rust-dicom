# DICOM Sender

A high-performance, multi-threaded DICOM C-STORE sender written in Rust using the `dicom-rs` crate.

## Features

- 📁 **Flexible Input**: Accept single files or directories (with recursive scanning)
- 🔍 **Smart Indexing**: Automatically parse DICOM headers and group by Study Instance UID
- 🚀 **Multi-threaded**: Concurrent transfers with configurable thread count
- 📊 **Detailed Metrics**: Comprehensive timing and transfer statistics
- 📋 **Logging**: Both detailed logs and JSON summary reports
- 🎨 **Progress Display**: Beautiful console progress indicators
- 🔒 **Study-based Associations**: One association per study for optimal performance

## Installation

```bash
cargo build --release
```

## Usage

### Basic Usage

```bash
# Send a single DICOM file
./target/release/dicom-sender -i /path/to/file.dcm -a DEST_AE -H 192.168.1.100 -p 4242

# Send all DICOM files in a directory
./target/release/dicom-sender -i /path/to/dicom/files -a DEST_AE -H 192.168.1.100 -p 4242

# Recursive directory scan with multiple threads
./target/release/dicom-sender -i /path/to/dicom -r -t 4 -a DEST_AE -H 192.168.1.100 -p 4242
```

### Command Line Options

```
Options:
  -i, --input <INPUT>              Input path (file or directory)
  -r, --recursive                  Recursive directory scanning
  -c, --calling-ae <CALLING_AE>    Called AE Title (default: RUST_SCU) [default: RUST_SCU]
  -a, --ae-title <AE_TITLE>        Called AE Title (destination)
  -H, --host <HOST>                Destination IP address
  -p, --port <PORT>                Destination port
  -t, --threads <THREADS>          Number of concurrent threads/associations [default: 1]
  -v, --verbose                    Verbose output
  -h, --help                       Print help
  -V, --version                    Print version
```

### Examples

#### Send CT Study with 4 threads
```bash
./target/release/dicom-sender \
  -i /data/ct_study \
  -r \
  -t 4 \
  -c "RUST_SENDER" \
  -a "PACS_SERVER" \
  -H 10.0.0.50 \
  -p 11112
```

#### Send specific files with verbose logging
```bash
./target/release/dicom-sender \
  -i /data/mr_series \
  -v \
  -a "MR_ARCHIVE" \
  -H pacs.hospital.com \
  -p 4242
```

## Architecture

### Study-based Associations
The sender intelligently groups DICOM files by Study Instance UID and opens one association per study. This approach:
- Minimizes association overhead
- Ensures logical grouping of related files
- Optimizes network usage

### Multi-threading Strategy
Each thread handles multiple studies sequentially:
```
Thread 1: Study A → Study B → Study C
Thread 2: Study D → Study E → Study F
Thread 3: Study G → Study H → Study I
```

### File Processing Pipeline
1. **Index Phase**: Scan input paths and parse DICOM headers
2. **Group Phase**: Group files by Study Instance UID
3. **Distribute Phase**: Distribute studies across worker threads
4. **Transfer Phase**: Send files using DICOM C-STORE
5. **Report Phase**: Generate logs and statistics

## Output

### Console Output
```
🚀 DICOM Sender v1.0
Session ID: 550e8400-e29b-41d4-a716-446655440000
Log file: dicom_sender_550e8400-e29b-41d4-a716-446655440000.log

📋 Indexing DICOM files...
✅ Found 150 DICOM files
📊 Grouped into 5 studies
  Study: 1.2.840.113619.2.55... (30 files)
  Study: 1.2.840.113619.2.56... (45 files)
  Study: 1.2.840.113619.2.57... (25 files)
  Study: 1.2.840.113619.2.58... (35 files)
  Study: 1.2.840.113619.2.59... (15 files)

🚀 Starting transfer with 2 threads...
✨ [00:02:15] [████████████████████████████████] 150/150 (00:00)

⏱️ Transfer Summary
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total files:     150
Successful:      148
Failed:          2
Total size:      2.34 GB
Total time:      135.45 seconds
Avg transfer:    903.2 ms
Throughput:      17.3 MB/s
Threads used:    2
Studies:         5

📄 Detailed log: dicom_sender_550e8400-e29b-41d4-a716-446655440000.log
📊 Summary JSON: dicom_sender_summary_550e8400-e29b-41d4-a716-446655440000.json
```

### Log Files

#### Detailed Log
Plain text log with timestamp, level, and detailed transfer information:
```
2024-09-10T15:30:15.123Z INFO  Thread 0 starting with 3 studies
2024-09-10T15:30:15.456Z INFO  Opening association for study: 1.2.840.113619.2.55...
2024-09-10T15:30:15.789Z INFO  Successfully sent file: /data/ct/image001.dcm
```

#### Summary JSON
Machine-readable summary with complete statistics:
```json
{
  "session_id": "550e8400-e29b-41d4-a716-446655440000",
  "start_time": "2024-09-10T15:30:15.000Z",
  "end_time": "2024-09-10T15:32:30.450Z",
  "total_files": 150,
  "successful_transfers": 148,
  "failed_transfers": 2,
  "total_bytes": 2515631104,
  "total_time_ms": 135450,
  "average_transfer_time_ms": 903.2,
  "throughput_mbps": 17.3,
  "threads_used": 2,
  "destination": "PACS_SERVER:4242@10.0.0.50",
  "calling_ae": "RUST_SENDER",
  "called_ae": "PACS_SERVER",
  "studies_processed": [
    "1.2.840.113619.2.55.3.2.1.1.7",
    "1.2.840.113619.2.56.3.2.1.1.8",
    "1.2.840.113619.2.57.3.2.1.1.9",
    "1.2.840.113619.2.58.3.2.1.1.10",
    "1.2.840.113619.2.59.3.2.1.1.11"
  ]
}
```

## Performance Tuning

### Thread Count
- **Single thread**: Good for testing and simple transfers
- **2-4 threads**: Optimal for most scenarios
- **8+ threads**: May overwhelm the destination server

### Network Considerations
- Monitor network bandwidth utilization
- Consider destination server capacity
- Adjust thread count based on file sizes

### File Organization
The sender performs best when:
- Files are grouped by study on disk
- DICOM headers are valid and complete
- Network latency is low

## Dependencies

- `dicom` - Core DICOM functionality
- `dicom-ul` - DICOM Upper Layer protocol
- `tokio` - Async runtime
- `clap` - Command line parsing
- `indicatif` - Progress bars
- `tracing` - Structured logging
- `serde` - JSON serialization

## Building from Source

```bash
git clone <repository>
cd dicom-sender
cargo build --release
```

## License

[Add your license here]

## Contributing

[Add contributing guidelines here]
