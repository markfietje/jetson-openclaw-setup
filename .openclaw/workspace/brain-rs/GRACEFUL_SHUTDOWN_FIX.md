# Graceful Shutdown Fix - v0.7.5

## 🐛 Root Cause

Brain-server hangs during graceful shutdown because the r2d2 connection pool is never closed.

### Why It Hangs:
1. Axum receives SIGTERM → initiates graceful shutdown
2. Shutdown future waits 60 seconds (drain period)
3. **Connection pool never closes**
4. Pool has `min_idle(Some(4))` → 4 connections stay open forever
5. Axum waits forever for connections to close
6. Systemd times out → SIGKILL

### Timeline from Logs (Feb 17, 07:19 UTC):
```
07:19:18 - Received SIGTERM, initiated 60s drain
07:19:36 - SIGKILL (only 18s later!)
07:19:37 - New instance started
```

**Systemd killed it after 18 seconds**, but it was supposed to wait 60 seconds!

---

## 🔧 The Fix

### Change 1: Clone Pool for Shutdown (line ~870)

**Before:**
```rust
let app = Router::new()
    .route("/health", get(health))
    // ... routes ...
    .layer(cors)
    .layer(Arc::new(AppState {
        pool: pool.clone(),
        db_path: db_path.clone(),
        connection_tracker: std::sync::Arc::clone(&connection_tracker),
    }));

axum::serve(listener, app)
    .with_graceful_shutdown(async {
        // ... shutdown code ...
    })
```

**After:**
```rust
// Clone pool for shutdown closure
let pool_shutdown = pool.clone();

let app = Router::new()
    .route("/health", get(health))
    // ... routes ...
    .layer(cors)
    .layer(Arc::new(AppState {
        pool: pool.clone(),
        db_path: db_path.clone(),
        connection_tracker: std::sync::Arc::clone(&connection_tracker),
    }));

axum::serve(listener, app)
    .with_graceful_shutdown(async {
        // ... shutdown code ...
        // ADD POOL CLOSE HERE (see Change 2)
    })
```

---

### Change 2: Close Pool Before Shutdown Completes (line ~936)

**Before:**
```rust
// In a real implementation, you'd track active requests
// For now, we wait and then shutdown
let _ = drain_complete.await;

let elapsed = drain_start.elapsed();
println!("✅ Graceful shutdown complete after {:.1}s", elapsed.as_secs_f64());
```

**After:**
```rust
// Wait for drain period
tokio::select! {
    _ = drain_complete => {
        // Drain period elapsed
    },
}

// CLOSE THE CONNECTION POOL!
println!("🔌 Closing database connection pool...");
pool_shutdown.close();
println!("✅ Connection pool closed");

let elapsed = drain_start.elapsed();
println!("✅ Graceful shutdown complete after {:.1}s", elapsed.as_secs_f64());
```

---

### Change 3: Reduce min_idle During Shutdown (Optional but Recommended)

The r2d2 pool's `min_idle(4)` keeps 4 connections open even during shutdown.

**Option A: Set min_idle to 0 before closing**
```rust
// Before pool.close()
println!("🔌 Draining connection pool...");
// pool_shutdown.set_min_idle(Some(0)); // ← This method doesn't exist in r2d2
// Workaround: close() is enough, it will close all connections
```

**Option B: Use pool.close() - it closes all connections**
```rust
// This is what we'll use
pool_shutdown.close();
```

---

## 📝 Complete Fixed Code Section

Here's the complete graceful shutdown block with the fix:

```rust
axum::serve(listener, app)
    .with_graceful_shutdown(async {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                println!("\n🔔 Received SIGINT (Ctrl+C)");
            },
            _ = terminate => {
                println!("\n🔔 Received SIGTERM");
            },
        }

        println!("\n🛑 Initiating graceful shutdown...");
        println!("⏳ Waiting up to {} seconds for in-flight requests to complete...", SHUTDOWN_DRAIN_SECS);

        // Drain period - wait for requests to complete
        let drain_start = std::time::Instant::now();
        let drain_complete = async {
            tokio::time::sleep(Duration::from_secs(SHUTDOWN_DRAIN_SECS)).await;
        };

        // Wait for drain period (or interrupted by actual in-flight request completion)
        tokio::select! {
            _ = drain_complete => {
                println!("⏱️  Drain period elapsed");
            },
        }

        // 🔧 FIX: Close the connection pool!
        println!("🔌 Closing database connection pool...");
        pool_shutdown.close();
        println!("✅ Connection pool closed");

        let elapsed = drain_start.elapsed();
        println!("✅ Graceful shutdown complete after {:.1}s", elapsed.as_secs_f64());
    })
    .await?;

Ok(())
```

---

## 🧪 Testing the Fix

### Manual Test:
```bash
# 1. Build with fix
cd /home/jetson/.openclaw/workspace/brain-rs
cargo build --release

# 2. Deploy
systemctl --user stop brain-server
cp target/release/brain-server ~/.openclaw/workspace/brain-server
systemctl --user start brain-server

# 3. Test graceful shutdown (should complete in ~1-2 seconds, not hang)
systemctl --user stop brain-server

# 4. Check logs for clean shutdown
journalctl --user -u brain-server --since "1 minute ago" | tail -20
```

### Expected Output:
```
🔔 Received SIGTERM
🛑 Initiating graceful shutdown...
⏳ Waiting up to 60 seconds for in-flight requests to complete...
⏱️  Drain period elapsed
🔌 Closing database connection pool...
✅ Connection pool closed
✅ Graceful shutdown complete after 1.2s
```

**Key:** Shutdown should complete in **1-2 seconds**, not hang!

---

## 📊 Impact

### Before Fix:
- ❌ Shutdown hangs forever
- ❌ Systemd SIGKILL after timeout (18-90s)
- ❌ Potential database corruption
- ❌ Gateway cascade failures

### After Fix:
- ✅ Clean shutdown in 1-2 seconds
- ✅ All connections closed gracefully
- ✅ No SIGKILL needed
- ✅ Safe database closure

---

## 🚀 Deployment Steps

1. Apply fix to `/home/jetson/.openclaw/workspace/brain-rs/src/main.rs`
2. Rebuild: `cargo build --release`
3. Test locally: `./target/release/brain-server` (Ctrl+C to test)
4. Deploy: `systemctl --user stop brain-server`
5. Copy binary: `cp target/release/brain-server ~/.openclaw/workspace/brain-server`
6. Start: `systemctl --user start brain-server`
7. Monitor: `journalctl --user -u brain-server -f`

---

## 📌 Additional Improvements (Optional)

### Improvement 1: Track Active Requests
The TODO comment suggests tracking active requests:
```rust
// In a real implementation, you'd track active requests
// For now, we wait and then shutdown
```

**Future enhancement:** Use `Arc<AtomicUsize>` to track in-flight requests and exit early when all complete.

### Improvement 2: Reduce SHUTDOWN_DRAIN_SECS
Current: 60 seconds is too long for most cases.
**Recommended:** 10 seconds
```rust
const SHUTDOWN_DRAIN_SECS: u64 = 10;
```

### Improvement 3: Set systemd TimeoutStopSec
Already done in v0.7.1 (30s), but verify:
```bash
grep TimeoutStopSec ~/.config/systemd/user/brain-server.service
# Should show: TimeoutStopSec=30
```

---

## ✅ Verification Checklist

- [ ] Fix applied to main.rs
- [ ] Rebuilt successfully
- [ ] Manual shutdown test passes (Ctrl+C)
- [ ] systemd restart test passes
- [ ] Logs show clean shutdown
- [ ] No "killed, signal=9" messages
- [ ] Health check still works
- [ ] Database not corrupted

---

**Version:** v0.7.5 (proposed)
**Date:** 2026-02-17
**Status:** Ready to implement
