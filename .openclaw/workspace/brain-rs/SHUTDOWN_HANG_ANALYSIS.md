# Brain-Server Shutdown Hang - Root Cause Analysis & Solution

## 📊 Incident Summary

**Date:** February 17, 2026, 07:19 UTC
**Component:** brain-server v0.7.4
**Symptom:** Service stuck in "deactivating (stop-sigterm)" state
**Required Action:** Manual SIGKILL to terminate

---

## 🔍 Timeline Analysis

```
07:19:18 UTC - brain-server[21173]: 🔔 Received SIGTERM
07:19:18 UTC - brain-server[21173]: 🛑 Initiating graceful shutdown...
07:19:18 UTC - brain-server[21173]: ⏳ Waiting up to 60 seconds for in-flight requests...
07:19:18 UTC - systemd[3458]: Stopping Brain Server v0.7.2 (Rust/NEON)...
07:19:36 UTC - systemd[3458]: brain-server.service: Main process exited, code=killed, status=9/KILL
07:19:36 UTC - systemd[3458]: brain-server.service: Failed with result 'signal'
```

**Key Observation:** Killed after only **18 seconds**, despite 60-second drain period!

---

## 🐛 Root Cause Identified

### **Primary Bug: Connection Pool Never Closes**

**Location:** `/home/jetson/.openclaw/workspace/brain-rs/src/main.rs` (lines 897-940)

**The Problem:**
```rust
axum::serve(listener, app)
    .with_graceful_shutdown(async {
        // Signal handling...
        tokio::time::sleep(Duration::from_secs(60)).await;
        println!("✅ Graceful shutdown complete");
        // ← BUG: Connection pool NEVER closes!
    })
```

**Why This Causes Hang:**
1. Axum initiates graceful shutdown when SIGTERM received
2. Shutdown future starts, waits 60 seconds
3. **Connection pool (r2d2) is never closed**
4. Pool configured with `min_idle(Some(4))` → 4 connections stay open
5. Axum's graceful shutdown waits **forever** for connections to close
6. Systemd waits 30 seconds (TimeoutStopSec=30)
7. Systemd sends SIGKILL to force termination

---

## 🔬 Technical Deep Dive

### **Connection Pool Configuration:**
```rust
let pool = r2d2::Pool::builder()
    .max_size(20)
    .min_idle(Some(4))              // ← KEEPS 4 CONNECTIONS OPEN
    .connection_timeout(Duration::from_secs(30))
    .max_lifetime(Some(Duration::from_secs(60)))
    .idle_timeout(Some(Duration::from_secs(20)))
    .test_on_check_out(true)
    .build(SqliteConnectionManager::file(&db_path))?;
```

**Issue:** The `min_idle(Some(4))` setting maintains **4 idle connections permanently**. These connections:
- Are held by r2d2 pool
- Never released during shutdown
- Prevent Axum from completing graceful shutdown
- Cause indefinite hang

### **Systemd Configuration:**
```ini
[Service]
TimeoutStopSec=30          # ← Systemd SIGKILL after 30s
ExecStartPre=/bin/sleep 2  # ← Wait for network (GOOD: no more pkill -9)
```

**Behavior:**
1. Sends SIGTERM (signal 15) → brain-server
2. Waits 30 seconds for graceful exit
3. Sends SIGKILL (signal 9) if still running
4. Result: "code=killed, status=9/KILL"

---

## 🔧 Solution

### **Fix: Close Pool During Graceful Shutdown**

**Two changes required:**

**1. Clone pool for shutdown closure (line ~870):**
```rust
// Add this before axum::serve()
let pool_shutdown = pool.clone();

let app = Router::new()
    // ... routes ...
    .layer(Arc::new(AppState {
        pool: pool.clone(),  // Original clone for app state
        // ...
    }));
```

**2. Close pool in shutdown future (line ~936):**
```rust
axum::serve(listener, app)
    .with_graceful_shutdown(async {
        // ... signal handling ...

        // Wait for drain period
        tokio::time::sleep(Duration::from_secs(SHUTDOWN_DRAIN_SECS)).await;

        // 🔧 FIX: Close the pool!
        println!("🔌 Closing database connection pool...");
        pool_shutdown.close();
        println!("✅ Connection pool closed");

        let elapsed = drain_start.elapsed();
        println!("✅ Graceful shutdown complete after {:.1}s", elapsed.as_secs_f64());
    })
```

---

## 📈 Expected Improvement

### **Before Fix:**
```
Shutdown Start (0s) → Wait forever → SIGKILL (18-30s) → Forced Termination
```

### **After Fix:**
```
Shutdown Start (0s) → Wait 60s drain → Close pool (0.1s) → Clean Exit (60.1s)
```

**Real-world:** With no active requests, shutdown completes in **1-2 seconds** (drain period can be interrupted).

---

## 🧪 Testing Plan

### **1. Manual Test (Ctrl+C):**
```bash
cd /home/jetson/.openclaw/workspace/brain-rs
cargo build --release
./target/release/brain-server
# Press Ctrl+C
# Expected: Clean shutdown in 1-2s
```

### **2. systemd Test:**
```bash
systemctl --user stop brain-server
# Check logs: journalctl --user -u brain-server --since "1 min ago" | tail -20
# Expected: No SIGKILL, clean exit
```

### **3. Restart Test:**
```bash
systemctl --user restart brain-server
# Expected: Clean stop, clean start, no cascade failures
```

---

## 🚀 Deployment Steps

1. **Apply fix to source:**
   ```bash
   cd /home/jetson/.openclaw/workspace/brain-rs
   # Edit src/main.rs with the two changes above
   ```

2. **Rebuild:**
   ```bash
   cargo build --release
   ```

3. **Deploy:**
   ```bash
   systemctl --user stop brain-server
   cp target/release/brain-server ~/.openclaw/workspace/brain-server
   systemctl --user start brain-server
   ```

4. **Verify:**
   ```bash
   systemctl --user status brain-server
   journalctl --user -u brain-server -n 20
   ```

---

## 📌 Additional Recommendations

### **1. Reduce Shutdown Drain Time**
**Current:** 60 seconds
**Recommended:** 10 seconds
**Reason:** Most requests complete in <1s, 60s is excessive

```rust
const SHUTDOWN_DRAIN_SECS: u64 = 10;  // Instead of 60
```

### **2. Track Active Requests (Future Enhancement)**
```rust
// Use Arc<AtomicUsize> to track in-flight requests
let active_requests = Arc::new(AtomicUsize::new(0));

// In each handler:
active_requests.fetch_add(1, Ordering::SeqCst);
defer! { active_requests.fetch_sub(1, Ordering::SeqCst); }

// In shutdown:
while active_requests.load(Ordering::SeqCst) > 0 {
    tokio::time::sleep(Duration::from_millis(100)).await;
}
```

### **3. Monitor Shutdown Time**
Add to health check logs:
```rust
println!("⏱️  Shutdown metrics:");
println!("   - Pool connections closed: {}", pool_shutdown.state().connections);
println!("   - Total time: {:.1}s", elapsed);
```

---

## ✅ Verification Checklist

- [ ] Fix applied to `src/main.rs`
- [ ] Rebuilt successfully (`cargo build --release`)
- [ ] Manual Ctrl+C test passes
- [ ] systemd restart test passes
- [ ] No "killed, signal=9" in logs
- [ ] Shutdown completes in <5 seconds (idle case)
- [ ] Database integrity verified
- [ ] Health check still works after restart
- [ ] No gateway cascade failures

---

## 📊 Impact Assessment

### **Risk:** Medium (requires code change + restart)
### **Effort:** Low (2-line fix, 5-minute deployment)
### **Benefit:** High (eliminates shutdown hangs, prevents cascade failures)
### **Urgency:** High (affects reliability, requires manual intervention)

---

## 🎯 Conclusion

**Root Cause:** r2d2 connection pool never closes during graceful shutdown
**Fix:** Add `pool.close()` before shutdown future completes
**Deployment:** v0.7.5 (recommended)
**Testing:** Manual + systemd restart tests
**Rollback:** Keep v0.7.4 backup until verified

---

**Document Version:** 1.0
**Created:** 2026-02-17 07:40 UTC
**Author:** Loeki (OpenClaw Assistant)
**Status:** Ready for implementation
