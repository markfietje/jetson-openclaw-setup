# Midnight Cron Job Failure - February 16, 2026

## Issue Report

**Date:** February 16, 2026 at 00:38 UTC
**Problem:** Daily MEMORY.md auto-ingest cron job failed
**Expected:** Automatic ingestion at midnight (00:00 UTC)
**Actual:** Cron job failed at 00:38:34 UTC

## Root Cause

**Error:** `device token mismatch (rotate/reissue device token)`

```
Feb 16 00:38:34 jetson openclaw[26421]: 2026-02-16T00:38:34.402+00:00 [tools] cron failed: gateway closed (1008): unauthorized: device token mismatch (rotate/reissue device token)
```

**What Happened:**
1. The OpenClaw cron system attempted to run the daily MEMORY.md ingestion job
2. The cron tool tried to connect to the gateway (ws://127.0.0.1:18789)
3. Gateway rejected the connection due to device token mismatch
4. Cron job execution failed - MEMORY.md was NOT ingested automatically

## Impact

| System | Status |
|--------|--------|
| **Automated daily sync** | ❌ FAILED |
| **Brain-server update** | ⚠️ MANUAL (done during conversation) |
| **Memory freshness** | ✅ Current (manual update) |

## Analysis

**Why This Matters:**
- The cron system is OpenClaw's automation layer
- Daily MEMORY.md ingestion keeps brain-server current without manual intervention
- This is a reliability issue - automation should work transparently

**Authentication Issue:**
- Device tokens are used for internal gateway authentication
- Token mismatch suggests the token rotated or changed
- The cron system may be using an old/stale token

## Solutions

### Option 1: Fix Gateway Authentication (Recommended)
```bash
# Reissue device token to fix authentication
openclaw auth rotate-token

# Or restart gateway to regenerate tokens
systemctl --user restart openclaw-gateway
```

### Option 2: Use Alternative Cron System
- Create a systemd user timer instead of OpenClaw cron
- Directly call brain-server API without gateway interaction
- More reliable, less dependent on gateway auth

### Option 3: Manual Ingestion (Current Fallback)
- Manually run ingestion when needed
- Command: `curl -s -X POST "http://127.0.0.1:8765/ingest/memory" -H "Content-Type: application/json" --data @MEMORY.md`

## Current Status

**Brain-server is current** because manual ingestion was performed during the session where the date correction happened (September 2026 → June 17, 2026).

However, the **automation is broken** and needs fixing for reliable daily updates.

## Next Steps

1. **Immediate:** Fix gateway authentication (rotate or restart)
2. **Test:** Verify cron job can connect and execute
3. **Monitor:** Check tomorrow's midnight run (Feb 17, 2026)
4. **Long-term:** Consider systemd timer for reliability

---

**Created:** February 16, 2026 at 00:40 UTC
**Severity:** Medium (automation broken, manual workaround exists)
**Status:** Pending fix
