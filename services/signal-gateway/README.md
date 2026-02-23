# signal-gateway

**Lightweight Rust-based Signal daemon for OpenClaw on Jetson Nano.**

A minimal, memory-efficient Signal gateway using [presage](https://github.com/whisperfish/presage) - no Java required! This replaces `signal-cli` for resource-constrained devices.

## Why This Instead of signal-cli?

| Feature | signal-cli | signal-gateway (Rust) |
|---------|-----------|----------------------|
| Runtime | Java JVM (~200MB+) | Native binary (~10MB) |
| Startup | 3-5 seconds | <100ms |
| Memory | 150-300MB | 20-50MB |
| Dependencies | Java 11+ | None (static binary) |
| ARM Support | Slow (JIT) | Native speed |

---

## Quick Start

### 1. Build (first time takes 10-15 min on Jetson Nano)

```bash
cd ~/signal-gateway
cargo build --release
```

### 2. Create Config File

```bash
mkdir -p data/attachments

cat > config.yaml << 'YAML'
server:
  address: "127.0.0.1:8080"

signal:
  data_dir: "./data"
  attachments_dir: "./data/attachments"
YAML
```

### 3. Link Your Phone

```bash
./target/release/signal-gateway link --device-name "OpenClaw"
```

**Output:**
```
Generating link URL for device: OpenClaw
Scan this QR code with your Signal app:
sgnl://linkdevice?uuid=xxxx&pub_key=yyyy

Or open the URL: sgnl://linkdevice?uuid=xxxx&pub_key=yyyy
```

**To complete linking:**
1. Open Signal on your phone
2. Go to **Settings → Linked Devices**
3. Tap **+** or "Link new device"
4. Scan the QR code OR manually enter the URL

### 4. Start the Server

```bash
./target/release/signal-gateway serve

# Output:
# Signal gateway listening on 127.0.0.1:8080
```

### 5. Test It Works

```bash
curl http://localhost:8080/api/v1/check
# {"status":"ok","version":"0.1.0"}
```

---

## OpenClaw Integration

The signal-gateway provides a **signal-cli compatible JSON-RPC API**, making it a drop-in replacement for OpenClaw's Signal channel.

### Architecture

```
+------------------+      JSON-RPC/SSE      +------------------+
|   OpenClaw       | <--------------------->|  signal-gateway  |
|   Gateway        |   http://127.0.0.1:8080 |   (Rust/presage) |
+------------------+                        +--------+---------+
                                                     |
                                                     v
                                            +----------------+
                                            | Signal Servers |
                                            | (presage lib)  |
                                            +----------------+
```

### Step 1: Start signal-gateway as a Service

Create a systemd service for auto-start:

```bash
sudo nano /etc/systemd/system/signal-gateway.service
```

```ini
[Unit]
Description=Signal Gateway for OpenClaw
After=network.target

[Service]
Type=simple
User=jetson
WorkingDirectory=/home/jetson/signal-gateway
ExecStart=/home/jetson/signal-gateway/target/release/signal-gateway serve
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable signal-gateway
sudo systemctl start signal-gateway

# Check status
sudo systemctl status signal-gateway
```

### Step 2: Configure OpenClaw Gateway

Edit your OpenClaw gateway configuration (`~/.openclaw/gateway.json`):

```json5
{
  agent: {
    workspace: "~/.openclaw/workspace"
  },
  channels: {
    signal: {
      enabled: true,
      // Point to signal-gateway HTTP endpoint
      cliPath: "http://127.0.0.1:8080",
      // Your linked Signal number (set after linking)
      account: "+15551234567",
      // Security: require pairing for unknown senders
      dmPolicy: "pairing",
      // Allowed senders (add your contacts)
      allowFrom: ["+15557654321"]
    }
  }
}
```

**Alternatively via OpenClaw CLI:**

```bash
# Add Signal channel
openclaw channels add signal --account "+15551234567"

# Set the gateway URL
openclaw channels config signal cliPath "http://127.0.0.1:8080"

# Set allowed senders
openclaw channels config signal allowFrom '["+15557654321"]'

# Enable the channel
openclaw channels enable signal
```

### Step 3: Restart OpenClaw Gateway

```bash
# If running as daemon
openclaw gateway restart

# Or manually
openclaw gateway --port 18789
```

### Step 4: Verify Integration

```bash
# Check OpenClaw channel status
openclaw channels status

# Should show:
# signal: connected (+15551234567)
```

### How OpenClaw Communicates

| OpenClaw Action | signal-gateway Endpoint |
|-----------------|------------------------|
| Send message | `POST /api/v1/rpc` method `sendMessage` |
| Receive messages | `GET /api/v1/events` (SSE stream) |
| Get account info | `POST /api/v1/rpc` method `getAccountNumber` |
| Typing indicator | `POST /api/v1/rpc` method `sendTyping` |
| React to message | `POST /api/v1/rpc` method `sendReaction` |

---

## API Reference

### JSON-RPC 2.0 Endpoint

All methods via `POST /api/v1/rpc`:

```bash
curl -X POST http://localhost:8080/api/v1/rpc \
  -H 'Content-Type: application/json' \
  -d '{
    "jsonrpc": "2.0",
    "method": "<METHOD>",
    "params": {...},
    "id": 1
  }'
```

### Available Methods

| Method | Params | Description |
|--------|--------|-------------|
| `sendMessage` | `recipient`, `message` | Send text message |
| `getAccountNumber` | *(none)* | Get linked phone number |
| `subscribeReceive` | *(none)* | Check for new messages |
| `listGroups` | *(none)* | List groups (stub: returns []) |
| `sendTyping` | `recipient` | Send typing indicator |
| `sendReadReceipt` | `recipient` | Mark as read (stub) |
| `sendReaction` | `recipient`, `targetTimestamp`, `emoji`, `remove` | React to message |

### Examples

#### Send Message

```bash
curl -X POST http://localhost:8080/api/v1/rpc \
  -H 'Content-Type: application/json' \
  -d '{
    "jsonrpc": "2.0",
    "method": "sendMessage",
    "params": {
      "recipient": "+15551234567",
      "message": "Hello from OpenClaw!"
    },
    "id": 1
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "timestamp": 1708556400000,
    "messageId": "550e8400-e29b-41d4-a716-446655440000"
  },
  "id": 1
}
```

#### Send Reaction

```bash
curl -X POST http://localhost:8080/api/v1/rpc \
  -H 'Content-Type: application/json' \
  -d '{
    "jsonrpc": "2.0",
    "method": "sendReaction",
    "params": {
      "recipient": "+15551234567",
      "targetTimestamp": 1708556400000,
      "emoji": "👍",
      "remove": false
    },
    "id": 1
  }'
```

#### Get Account Number

```bash
curl -X POST http://localhost:8080/api/v1/rpc \
  -H 'Content-Type: application/json' \
  -d '{
    "jsonrpc": "2.0",
    "method": "getAccountNumber",
    "params": {},
    "id": 1
  }'
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {"number": "+15551234567"},
  "id": 1
}
```

### Real-time Events (SSE)

Connect to receive incoming messages:

```bash
curl -N http://localhost:8080/api/v1/events
```

**JavaScript Example:**

```javascript
const eventSource = new EventSource('http://localhost:8080/api/v1/events');

eventSource.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Received:', data);
  // data = {
  //   source: "+15551234567",
  //   timestamp: 1708556400000,
  //   dataMessage: {
  //     timestamp: 1708556400000,
  //     message: "Hello!",
  //     attachments: []
  //   }
  // }
};

eventSource.onerror = (err) => {
  console.error('SSE error:', err);
};
```

**Note:** The gateway doesn't automatically poll for messages. Call `subscribeReceive` periodically or use the SSE stream.

---

## CLI Reference

```
signal-gateway <COMMAND>

Commands:
  serve   Start the HTTP server
  link    Link a new device

Options:
  -h, --help  Print help

Serve Options:
  -c, --config <FILE>  Config file path [default: config.yaml]

Link Options:
  -c, --config <FILE>    Config file [default: config.yaml]
      --device-name <N>  Device name in Signal [default: openclaw-gateway]
```

---

## Directory Structure

```
~/signal-gateway/
├── target/release/signal-gateway  # Binary (~10.5MB)
├── config.yaml                     # Configuration
├── data/
│   ├── signal.db                   # Signal keys & state (BACKUP THIS!)
│   └── attachments/                # Downloaded attachments
```

---

## Configuration Reference

```yaml
# config.yaml

server:
  # Bind address (use 127.0.0.1 for local-only, 0.0.0.0 for network access)
  address: "127.0.0.1:8080"

signal:
  # Directory for Signal database and keys
  data_dir: "./data"
  # Directory for attachment storage
  attachments_dir: "./data/attachments"
```

---

## Jetson Nano Optimization

Built specifically for ARM Cortex-A57:

| Setting | Value | Purpose |
|---------|-------|---------|
| Target CPU | cortex-a57 | Jetson Nano CPU |
| Features | +neon | SIMD acceleration |
| LTO | fat | Maximum optimization |
| Codegen Units | 1 | Best optimization |
| Strip | symbols | Smaller binary |
| Panic | abort | No unwinding overhead |
| **Binary Size** | ~10.5 MB | vs 200MB+ for Java |

---

## Security Notes

1. **No Authentication** - The gateway has no built-in auth. Protect with:
   - Firewall (bind to 127.0.0.1)
   - Tailscale/VPN for remote access
   - Reverse proxy with auth

2. **Backup `data/signal.db`** - Contains your Signal identity keys. If lost, you must re-link.

3. **Directory Permissions**:
   ```bash
   chmod 700 data
   chmod 600 data/signal.db
   ```

4. **OpenClaw dmPolicy** - Always use `"pairing"` mode:
   ```json5
   dmPolicy: "pairing"  // Unknown senders must pair first
   ```

---

## Troubleshooting

### "Not linked" error

Run the link command:
```bash
./signal-gateway link --device-name "MyDevice"
```

### Linking times out

The linking URL expires in ~2 minutes. Generate a fresh one.

### Messages not received

The gateway requires explicit polling:
```bash
# Trigger message check
curl -X POST http://localhost:8080/api/v1/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"subscribeReceive","params":{},"id":1}'
```

Or use the SSE stream for real-time updates.

### OpenClaw can't connect

1. Verify signal-gateway is running:
   ```bash
   curl http://localhost:8080/api/v1/check
   ```

2. Check OpenClaw channel config:
   ```bash
   openclaw channels status
   ```

3. Ensure `cliPath` points to the HTTP URL, not a binary path.

### Permission denied on data directory

```bash
mkdir -p data/attachments
chmod 755 data data/attachments
```

---

## Building from Source

```bash
# Requires Rust 1.70+
cd ~/signal-gateway

# Build (10-15 min on Jetson Nano)
cargo build --release

# Binary location
ls -la target/release/signal-gateway
```

---

## License

MIT OR Apache-2.0
