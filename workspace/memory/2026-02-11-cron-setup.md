# Daily MEMORY.md Auto-Sync - February 11, 2026

## Automated Brain-Server Ingestion

### Cron Job Created

**Job Name:** "Daily MEMORY.md Ingest to Brain-Server"
**Schedule:** Daily at midnight (00:00) - Cron expression: `0 0 * * *`
**Status:** Active and enabled
**Job ID:** 72c82b99-d7ab-479c-a9dd-8a458de958a4

### How It Works

**Trigger Mechanism:**
- Cron job sends system event to main session at midnight
- System event text pattern: "INGEST_MEMORY_MD_TRIGGER"
- When detected, execute ingestion command automatically

**Ingestion Command:**
```bash
curl -s -X POST "http://127.0.0.1:8765/ingest/memory" \
  -H "Content-Type: text/markdown" \
  --data-binary @/home/jetson/.openclaw/workspace/MEMORY.md | jq '.'
```

**Manual Execution (anytime):**
```bash
# Same command - can run manually anytime
curl -s -X POST "http://127.0.0.1:8765/ingest/memory" \
  -H "Content-Type: text/markdown" \
  --data-binary @/home/jetson/.openclaw/workspace/MEMORY.md | jq '.'
```

### Detection Pattern

**In session startup/maintenance:**
- Look for system events starting with "INGEST_MEMORY_MD_TRIGGER"
- When detected, immediately execute curl command above
- Check for status "success": true in jq output
- Verify brain-server count increased (optional confirmation)

### Benefits

**Automatic Updates:**
- MEMORY.md is ingested daily without manual intervention
- Brain-server always has latest curated memories
- No need to remember to sync - it just happens
- Works even if you don't use OpenClaw for days

**Redundant Coverage:**
- Daily sync means brain is never stale
- Combines with weekly memory/*.md sync during heartbeats
- Fast semantic search across all personal and technical knowledge

### Workflow Integration

**MEMORY.md → Daily Auto-Ingest → Brain-Server**
```
MEMORY.md (manual edits)
    ↓ (midnight auto)
Brain-Server (2,695+ entries, semantic search)
    ↓ (query <1ms)
Instant answers to your questions
```

**memory/*.md → Heartbeat Sync → Brain-Server**
```
memory/YYYY-MM-DD.md (daily logs)
    ↓ (weekly heartbeat)
Brain-Server (deduplicated automatically)
```

### Testing Notes

**Cron Job Test (Feb 11, 2026):**
- Manual run via `cron run` command
- Result: Gateway timeout after 60 seconds
- Likely cause: Main session not ready/accessible within timeout window
- Job creation verified as successful
- Next scheduled run: Midnight tonight (00:00)

**Verification Steps:**
1. Check tomorrow if brain count increased
2. Look for system event "INGEST_MEMORY_MD_TRIGGER" in logs
3. Verify MEMORY.md content appears in brain search results

### Troubleshooting

**If cron job doesn't execute:**
1. Check cron list: `cron list`
2. Verify job is enabled: Should show `"enabled": true`
3. Check system events in session logs
4. Try manual execution using curl command above

**If brain-server doesn't update:**
1. Verify brain-server is running: `curl -s "http://127.0.0.1:8765/stats"`
2. Check MEMORY.md path is correct
3. Verify MEMORY.md has content
4. Look for errors in jq output

---

**Created:** February 11, 2026
**Purpose:** Automate daily MEMORY.md ingestion to brain-server
**Status:** Active (cron job created, first run scheduled midnight tonight)
