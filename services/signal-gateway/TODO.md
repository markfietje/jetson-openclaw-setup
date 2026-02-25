# Signal Gateway - Post-Build TODO

## Current Status
- Build has syntax errors (escaped characters in string literals)
- Worker.rs needs fixes for character comparisons

## Remaining Steps After Build

### 1. Fix Build Errors
The issues are in worker.rs line 58 and 63 - string literal escaping.

### 2. Deploy New Binary
```bash
sudo systemctl stop signal-gateway
sudo cp ~/signal-gateway/target/release/signal-gateway /usr/local/bin/
sudo systemctl start signal-gateway
```

### 3. Seed the Recipient Cache
```bash
curl -X POST http://localhost:8080/v1/cache/seed \
  -H "Content-Type: application/json" \
  -d "{\"phone\": \"+15551234567\", \"uuid\": \"1c46c936-2fb7-42e5-818e-a3f20da4627d\"}"
```

### 4. Test Sending
```bash
# Test with phone number
curl -X POST http://localhost:8080/api/v1/rpc \
  -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"sendMessage\",\"params\":{\"recipient\":\"+15551234567\",\"message\":\"Test!\"},\"id\":1}"

# Test with UUID directly  
curl -X POST http://localhost:8080/api/v1/rpc \
  -H "Content-Type: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"method\":\"sendMessage\",\"params\":{\"recipient\":\"1c46c936-2fb7-42e5-818e-a3f20da4627d\",\"message\":\"Test!\"},\"id\":1}"
```

### 5. Restart OpenClaw Gateway
```bash
sudo systemctl restart openclaw-gateway
```

### 6. Test End-to-End
1. Send message from phone to OpenClaw-Jetson
2. Verify OpenClaw receives it
3. Reply from OpenClaw
4. Verify reply on phone

## Recipient Formats Supported
| Format | Example | Status |
|--------|---------|--------|
| UUID | 1c46c936-2fb7-42e5-818e-a3f20da4627d | Works |
| Phone (cached) | +15551234567 | Works after seeding |
| Username | markfietje.77 | Not implemented |

## Key Files
- /usr/local/bin/signal-cli - Wrapper script
- /usr/local/bin/signal-gateway - Rust binary
- /etc/signal-gateway/config.yaml - Config
- /var/lib/signal-gateway/signal.db - Database

## Future Enhancements (Not Yet Implemented)

### 1. Username Resolution via /v1/accounts/username_hash API
Signal supports username-based messaging (for privacy). Implementation needed:

```rust
// In worker.rs - resolve_username function
async fn resolve_username(&self, username: &str) -> Result<String> {
    // 1. Hash username with Ristretto 25519
    // 2. Call GET /v1/accounts/username_hash/{hash}
    // 3. Returns ACI UUID
    // 4. Cache result
}
```

Reference: `libsignal/rust/net/chat/src/ws/usernames.rs`

### 2. Phone Number Resolution (Contacts/Self-Reference)
Current implementation only caches self phone→UUID. Need:

- **CDS (Contact Discovery Service)** - Resolve any phone number to UUID
- **Contact sync** - Pull contacts from Signal and cache mappings
- **Incoming message caching** - Auto-cache sender phone→UUID when receiving

```rust
// Use libsignal CDSI for contact discovery
// See: libsignal/rust/net/src/cdsi.rs
```

### 3. Update API Layer to Accept All Formats
Update `src/api/mod.rs` to normalize all recipient formats:

| Input Format | Example | Normalization |
|--------------|---------|---------------|
| Phone number | `+15551234567` | Resolve via cache/CDS |
| UUID | `1c46c936-...` | Pass through |
| Username | `markfietje.77` | Resolve via username_hash API |
| `u:` prefix | `u:1c46c936-...` | Strip prefix |

```rust
// In api/mod.rs - normalize_recipient function
fn normalize_recipient(recipient: &str) -> String {
    if recipient.starts_with("u:") {
        recipient[2..].to_string()
    } else if recipient.starts_with("+") {
        // Will be resolved by worker cache
        recipient.to_string()
    } else if recipient.contains(".") && !recipient.contains("-") {
        // Username format - needs resolution
        recipient.to_string()
    } else {
        recipient.to_string()
    }
}
```

## Implementation Priority

1. **High Priority** - Phone number auto-resolution (self-reference) ✅ Done in worker.rs
2. **Medium Priority** - API layer format normalization
3. **Low Priority** - Username resolution (requires Signal API changes)
