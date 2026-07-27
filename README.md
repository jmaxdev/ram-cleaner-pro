# RAM Purger Pro

![License: UPL 1.0](https://img.shields.io/badge/License-UPL%201.0-blue?style=for-the-badge)
![Non-Commercial](https://img.shields.io/badge/Non--Commercial-Only-red?style=for-the-badge)
 
RAM Purger Pro is a high-performance Windows system utility written in Rust. Designed for deep physical memory optimization, it purges system-wide Working Sets, Standby Lists, Modified Page Lists, and the System File Cache using native Windows NT Kernel APIs (`NtSetSystemInformation`).

## Overview

Unlike standard task managers that only terminate processes, RAM Purger Pro communicates directly with the Windows NT kernel to reclaim allocated but unreferenced physical memory pages. It provides both an intuitive, modern desktop interface and an automated background monitoring engine.

## Architecture and Core Purge Levels

RAM Purger Pro executes memory optimization across 4 distinct kernel levels:

1. **Working Sets Trimming**: Issues `MemoryEmptyWorkingSets` via `NtSetSystemInformation` (Class 80) and iterates through active processes using `K32EmptyWorkingSet` and `SetProcessWorkingSetSize`. By acquiring `SeDebugPrivilege` and `SeIncreaseQuotaPrivilege`, it forces working set reduction across all user, background, and system service processes.
2. **Standby Memory List Purge**: Dispatches `MemoryPurgeStandbyList` to reclaim cached pages in standby lists and restore them to the free physical memory pool.
3. **Modified Page List Flush**: Dispatches `MemoryFlushModifiedList` to flush dirty memory pages to disk storage, reducing overall memory commit charge.
4. **System File Cache Flush**: Executes `SystemFileCacheInformation` (`NtSetSystemInformation` Class 21) with maximum working set bounds to clear Windows file system buffers.

## System Requirements

- **Operating System**: Windows 10 or Windows 11 (64-bit edition).
- **Permissions**: Administrator privileges are mandatory to adjust process token privileges and execute NT Kernel calls.

## Recommended Usage Guidelines and Precautions

- **Administrative Execution**: The application must be launched with elevated privileges ("Run as Administrator"). Without administrative elevation, native NT API calls will return Access Denied errors (Error `0xC0000022` / OS Error 5).
- **Working Set Performance Considerations**: Frequent working set trimming forces applications to reload required code pages from storage into RAM upon regaining focus. Setting an excessively aggressive auto-purge frequency may lead to temporary application responsiveness degradation. It is recommended to maintain an interval of at least 30 minutes and a safety cooldown of at least 60 seconds.
- **Disk I/O Impact**: Flushing modified page lists forces unwritten dirty memory pages to be written to disk. During periods of heavy disk activity, triggering this level may temporarily increase disk latency.

## Command Line Interface (CLI) Usage

RAM Purger Pro provides full command-line capabilities for headless operation, automated scripting, scheduled tasks, and system diagnostics.

### Available Arguments and Flags

| Flag / Option | Long Syntax | Description |
| :--- | :--- | :--- |
| `-p` | `--purge-now` | Executes an immediate RAM memory purge via CLI and prints execution metrics before exiting. |
| `-s` | `--status` | Queries current memory usage statistics and outputs formatted JSON metrics. |
| `-d` | `--daemon` | Starts the application as a headless background monitoring service. |
| `-g` | `--gui` | Launches the graphical user interface dashboard (default behavior when no mode flag is passed). |
| | `--threshold <FLOAT>` | Temporarily overrides the RAM usage percentage threshold for auto-purging (e.g. `--threshold 80.0`). |
| | `--interval <INT>` | Temporarily overrides the auto-purge time interval in minutes (e.g. `--interval 15`). |

### Command Examples

```powershell
# Execute an immediate RAM memory purge (Run as Administrator required)
.\ram-pro.exe --purge-now

# Output current physical memory metrics in JSON format
.\ram-pro.exe --status

# Launch background daemon service with custom threshold (80%) and interval (15 min)
.\ram-pro.exe --daemon --threshold 80.0 --interval 15
```

## Configuration Parameters

Default configuration file is stored at `%APPDATA%\.ramcleaner\config.toml`:

| Parameter | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `auto_purge_enabled` | Boolean | `true` | Enables or disables the background automated purge engine. |
| `threshold_percent` | Float | `20.0` | Minimum RAM usage percentage required to trigger auto-purge (Min: 20.0%). |
| `interval_minutes` | Integer | `30` | Time duration between automated monitoring checks in minutes (Min: 7 min). |
| `cooldown_seconds` | Integer | `60` | Safety cooldown period between consecutive purges in seconds (Min: 30 sec). |
| `purge_working_sets` | Boolean | `true` | Toggles Level 1 process working set reduction. |
| `purge_standby_list` | Boolean | `true` | Toggles Level 2 standby list purging. |
| `purge_modified_list` | Boolean | `true` | Toggles Level 3 modified page list flushing. |
| `purge_system_cache` | Boolean | `true` | Toggles Level 4 system file cache purging. |

## Building from Source

```powershell
# Clone the repository
git clone https://github.com/jmaxdev/ram-purger.git
cd ram-purger

# Compile in release mode
cargo build --release
```

The optimized production executable will be located at:
`target/release/ram-pro.exe`

## License

Developed by **jmaxdev**. Distributed under the UPL-1.0 License.
