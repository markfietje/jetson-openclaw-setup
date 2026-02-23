# Hourly Brain-Server Update Schedule - Updated February 16, 2026

**Schedule:** HOURLY (every hour at :00 minutes past the hour)
**Frequency:** 24 times per day
**First run:** Next hour (e.g., if it's 00:58, first run is 01:00)
**Script:** `/home/jetson/.openclaw/workspace/scripts/hourly-memory-ingest.sh`

---

## ✅ CONFIGURATION

### **Crontab Entry:**
```bash
0 * * * * /home/jetson/.openclaw/workspace/scripts/hourly-memory-ingest.sh
```

**Runs:** At the top of every hour (00:00, 01:00, 02:00, ... 23:00)

### **Log Files:**
**Location:** `/home/jetson/.openclaw/logs/daily-ingest-YYYYMMDD.log`

**Example:**
```
=== DAILY MEMORY.MD INGEST ===
Timestamp: 2026-02-16 00:58:09 UTC
Testing brain-server...
{"count":907,"embeddings":2993}
Ingesting MEMORY.md...
{"added":0,"error":"10 duplicates skipped","status":"completed","success":true}
SUCCESS: MEMORY.md ingested successfully
=== END DAILY INGEST ===
```

---

## 📊 HOURLY vs DAILY COMPARISON

| Metric | Daily | Hourly | Change |
|--------|-------|--------|--------|
| **Update frequency** | 1x per day | 24x per day | **24x more frequent** ⚡ |
| **Max staleness** | 24 hours | 1 hour | **24x fresher** ✨ |
| **Resource usage** | Minimal | Minimal | No significant change |
| **Deduplication** | ✅ Smart | ✅ Smart | Same efficiency |
| **Brain-server load** | Very low | Very low | Negligible increase |

---

## 💡 WHY HOURLY MAKES SENSE

### **Pros:**
- ✅ **Very fresh knowledge** — Maximum 1 hour stale
- ✅ **Captures mid-day additions** — No need to wait until midnight
- ✅ **Still efficient** — Brain-server skips duplicates (xxHash3)
- ✅ **Minimal overhead** — ~1-2 second processing time
- ✅ **Better for active work** — Immediate access to new notes

### **Perfect For:**
- Active research and planning
- Frequent note-taking throughout the day
- Real-time documentation work
- Multi-session work days
- Capturing time-sensitive information

---

## 🕐 SCHEDULE

**Runs automatically:** Every hour at :00 minutes

**Example schedule (UTC):**
- 00:00 (midnight)
- 01:00 (1 AM)
- 02:00 (2 AM)
- ...
- 12:00 (noon)
- ...
- 23:00 (11 PM)

**Next run:** Next hour (e.g., if current time is 00:58, next run is 01:00)

---

## 🔧 MANAGEMENT

### **View crontab:**
```bash
crontab -l
```

### **View current log:**
```bash
tail -f /home/jetson/.openclaw/logs/daily-ingest-$(date +%Y%m%d).log
```

### **Test script manually:**
```bash
bash /home/jetson/.openclaw/workspace/scripts/hourly-memory-ingest.sh
```

### **Pause hourly updates (switch back to daily):**
```bash
echo "0 0 * * * /home/jetson/.openclaw/workspace/scripts/hourly-memory-ingest.sh" | crontab -
```

### **Disable entirely:**
```bash
crontab -r
```

---

## 📈 EXPECTED BEHAVIOR

### **What You'll See:**

**Most hours:**
```
{"added":0,"error":"15 duplicates skipped","status":"completed","success":true}
SUCCESS: MEMORY.md ingested successfully
```
**Translation:** No new content, everything already in brain ✅

**After you add notes:**
```
{"added":3,"error":"0 duplicates","status":"completed","success":true}
SUCCESS: MEMORY.md ingested successfully
```
**Translation:** 3 new chunks added to brain ✨

---

## 🎯 SUMMARY

**Updated:** February 16, 2026 at 00:58 UTC
**Schedule:** Hourly (every :00 minutes past the hour)
**Status:** ✅ Active and tested
**First automatic run:** Next hour
**Resource impact:** Negligible (smart deduplication)

---

**Your brain will now stay updated every hour!** 🧠⚡

**Maximum knowledge freshness with zero overhead!** ✨
