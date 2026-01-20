# ipdnsd

A lightweight daemon that monitors your IP addresses (internal and external) and automatically updates DNS records when changes are detected. Perfect for home servers, self-hosted services, and dynamic IP environments.

## Features

- **Automatic IP monitoring** - Detects changes to both internal (LAN) and external (public) IP addresses
- **Multiple DNS providers** - Currently supports GoDaddy, designed for easy extension
- **Secure credential storage** - Credentials stored in system config with restricted permissions
- **Cross-platform** - Works on Linux, macOS, and Windows
- **System service support** - Run as systemd, launchd, or Windows Service
- **Multiple DNS entries** - Update multiple domains/subdomains with a single daemon

## Quick Install

### Linux / macOS

```bash
curl -fsSL https://raw.githubusercontent.com/diginera/ipdnsd/main/scripts/install.sh | sh
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/diginera/ipdnsd/main/scripts/install.ps1 | iex
```

## Manual Installation

### From Releases

Download the latest binary for your platform from the [Releases](https://github.com/diginera/ipdnsd/releases) page.

### From Source

```bash
# Clone the repository
git clone https://github.com/diginera/ipdnsd.git
cd ipdnsd

# Build
cargo build --release

# Install (optional)
cargo install --path .
```

## Configuration

### 1. Store API Credentials

Store your DNS provider API credentials (saved to `/etc/ipdnsd/credentials.toml` on Linux/macOS or `C:\ProgramData\ipdnsd\credentials.toml` on Windows):

```bash
# For GoDaddy
ipdnsd set-key godaddy
# You will be prompted for API Key and API Secret
```

To get GoDaddy API credentials:
1. Go to https://developer.godaddy.com/keys
2. Create a new API key (Production environment)
3. Save both the Key and Secret

### 2. Create Configuration File

The config file location depends on your OS:
- **Linux/macOS**: `/etc/ipdnsd/config.toml`
- **Windows**: `C:\ProgramData\ipdnsd\config.toml`

Run `ipdnsd config` to see the exact path and create a default config.

Example configuration:

```toml
[daemon]
interval_seconds = 300  # Check every 5 minutes
log_level = "info"      # trace, debug, info, warn, error

# Update root domain with external IP
[[dns_entries]]
provider = "godaddy"
domain = "example.com"
record_name = "@"
record_type = "A"
ip_source = "external"

# Update subdomain with internal/LAN IP
[[dns_entries]]
provider = "godaddy"
domain = "example.com"
record_name = "internal"
record_type = "A"
ip_source = "internal"
ttl = 3600  # Optional: TTL in seconds
```

### 3. Test Configuration

```bash
# Check current IPs and DNS records
ipdnsd check
```

Example output:
```
Checking IP addresses...

External IP: 203.0.113.42
Internal IP: 192.168.1.100

Checking DNS records...

@.example.com (A): godaddy -> 203.0.113.42
internal.example.com (A): godaddy -> 192.168.1.100
```

## Running the Daemon

### Foreground (for testing)

```bash
ipdnsd daemon
```

Press `Ctrl+C` to stop.

### As a System Service

#### Linux (systemd)

```bash
# Install and start service
sudo ipdnsd install

# Check status
sudo systemctl status ipdnsd

# View logs
sudo journalctl -u ipdnsd -f

# Stop/restart
sudo systemctl stop ipdnsd
sudo systemctl restart ipdnsd

# Uninstall
sudo ipdnsd uninstall
```

#### macOS (launchd)

```bash
# Install and start service
sudo ipdnsd install

# Check if running
launchctl list | grep ipdnsd

# View logs
tail -f /var/log/ipdnsd.log

# Uninstall
sudo ipdnsd uninstall
```

#### Windows (Windows Service)

Run PowerShell as Administrator:

```powershell
# Install and start service
ipdnsd install

# Check status
Get-Service ipdnsd

# Stop/restart
Stop-Service ipdnsd
Restart-Service ipdnsd

# Uninstall
ipdnsd uninstall
```

## CLI Reference

```
ipdnsd <COMMAND>

Commands:
  daemon      Run the daemon to monitor IP changes and update DNS
  check       Check current IPs and DNS records
  set-key     Store API credentials for a DNS provider
  delete-key  Delete stored API credentials for a DNS provider
  config      Show configuration file location and contents
  install     Install as a system service
  uninstall   Uninstall the system service
  help        Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## How It Works

1. The daemon starts and loads your configuration
2. On each interval (default 5 minutes):
   - Fetches your current external IP from multiple services (ipify, ifconfig.me, etc.)
   - Gets your internal LAN IP from network interfaces
   - Compares with cached values to detect changes
   - If changed, queries the DNS provider for current record values
   - Updates DNS records only when the IP has actually changed
3. Logs all actions and continues monitoring

## Supported DNS Providers

| Provider | Status | API Docs |
|----------|--------|----------|
| GoDaddy  | ✅ Supported | [API Docs](https://developer.godaddy.com/doc/endpoint/domains) |
| Cloudflare | 🔜 Planned | - |
| AWS Route53 | 🔜 Planned | - |

## Troubleshooting

### "API key not found for provider"

Make sure you've stored your credentials:
```bash
ipdnsd set-key godaddy
```

### "Configuration file not found"

Create a config file at the path shown by:
```bash
ipdnsd config
```

### DNS record not updating

1. Check that your API credentials are correct
2. Verify the domain and record name in your config
3. Run `ipdnsd check` to see current values
4. Check the daemon logs for errors

### Permission denied when installing service

The `install` command requires administrator/root privileges:
- **Linux/macOS**: Use `sudo ipdnsd install`
- **Windows**: Run PowerShell as Administrator

## Building from Source

### Prerequisites

- Rust 1.70 or later
- For Windows: Visual Studio Build Tools

### Build

```bash
cargo build --release
```

### Run Tests

```bash
cargo test
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### Adding a New DNS Provider

1. Create a new file in `src/dns/` (e.g., `cloudflare.rs`)
2. Implement the `DnsProvider` trait
3. Add the provider to the factory function in `src/dns/mod.rs`
4. Update documentation

## License

MIT License - see [LICENSE](LICENSE) for details.
