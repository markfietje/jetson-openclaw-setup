# Jetson Backup & Sync Strategy

Professional backup and synchronization strategy for Jetson workspace.

## 🎯 Strategy: Hybrid Approach

**Keep on Jetson:**
- ✅ Critical runtime files (MEMORY.md, config.yaml)
- ✅ Brain database (brain.db)
- ✅ Active development files
- ✅ Current versions only

**Keep on GitHub (MacBook):**
- ✅ Source code (brain-rs, signal-gateway)
- ✅ Configuration files
- ✅ Documentation (MEMORY.md, HEARTBEAT.md, etc.)
- ✅ Deployment scripts
- ✅ Complete version history

**Remove from Jetson:**
- ❌ Old backups (>7 days old)
- ❌ Build artifacts (target/, debug/)
- ❌ Duplicate files
- ❌ Temporary files

---

## 📦 Three-Tier Backup System

### Tier 1: GitHub (Code + Config) - PRIMARY BACKUP

**Location:** https://github.com/markfietje/jetson-openclaw-setup

**What's backed up:**
- ✅ All source code
- ✅ Configuration files
- ✅ Documentation (MEMORY.md, HEARTBEAT.md, etc.)
- ✅ Deployment scripts
- ✅ Systemd service files
- ✅ CHANGELOG, README

**Frequency:** Every commit (automatic)

**Restore:**
```bash
git clone https://github.com/markfietje/jetson-openclaw-setup.git
```

### Tier 2: Jetson Local (Runtime Data) - WORKING COPY

**Location:** `~/.openclaw/workspace/`

**What's kept:**
- ✅ brain.db (latest only)
- ✅ MEMORY.md (working copy)
- ✅ config.yaml (active config)
- ✅ Current brain-server binary
- ✅ Current signal-gateway binary

**Cleanup:** Remove old files daily (cleanup script)

**Restore:** From GitHub + reinstall binaries

### Tier 3: MacBook (Development + Staging) - DEVELOPMENT ENVIRONMENT

**Location:** `~/Sites/jetson-openclaw-setup/`

**What's kept:**
- ✅ Complete monorepo
- ✅ All history
- ✅ CI/CD workflows
- ✅ Documentation
- ✅ Release artifacts

**Frequency:** Every push

**Restore:** Git has full history

---

## 🧹 Cleanup Routine

### Daily (Automated)
Run cleanup script:
```bash
~/scripts/cleanup-jetson-workspace.sh
```

**What it does:**
- Removes backups older than 7 days
- Removes build artifacts older than 7 days
- Removes log files older than 7 days
- Keeps only latest versions
- Creates safety backup before cleanup

### Weekly (Manual)
1. Review backup directory
2. Delete old backups if needed
3. Verify space usage
4. Check git status

### Monthly (Manual)
1. Full workspace cleanup
2. Archive old memory files
3. Review GitHub repo size
4. Test restore from backup

---

## 🔄 Sync Strategy

### Jetson → MacBook → GitHub

```
Jetson (Production)
  ↓ rsync/copy
MacBook (Development)
  ↓ git commit/push
GitHub (Backup/Version Control)
```

### When to Sync

**Before major changes:**
```bash
# On Jetson
~/.openclaw/workspace/scripts/sync-jetson-to-github.sh
```

**After important updates:**
```bash
# On Jetson
~/.openclaw/workspace/scripts/sync-jetson-to-github.sh

# On MacBook
cd ~/Sites/jetson-openclaw-setup
git add workspace/
git commit -m "chore: Sync workspace from Jetson"
git push origin main
```

---

## 📊 Space Management

### Current State (Before Cleanup)
```
~/.openclaw/workspace/: 2.2GB
  ├─ brain-rs/target/:     1.3GB  (can clean)
  ├─ brain-rs-backup/:      465MB  (can delete)
  ├─ Old backups:           70MB   (can delete)
  └─ Active files:          365MB  (KEEP!)

~/signal-gateway/target/:  1.2GB   (can clean)
```

**Waste:** ~2GB of old files!

### Target State (After Cleanup)
```
~/.openclaw/workspace/: 200MB
  ├─ brain.db:             21MB   (KEEP!)
  ├─ MEMORY.md:            20KB   (sync to GitHub)
  ├─ config.yaml:          1KB    (sync to GitHub)
  └─ Active files:         180MB  (KEEP!)

~/signal-gateway/target/:  200MB   (keep release only)
```

**Space saved:** ~2GB!

---

## 🛡️ Backup Checklist

### Before Cleanup:
- [ ] Run sync script to backup to GitHub
- [ ] Verify backup created successfully
- [ ] Check critical files (MEMORY.md, brain.db)

### After Cleanup:
- [ ] Verify services still running
- [ ] Check brain database integrity
- [ ] Test OpenClaw functionality
- [ ] Review cleanup log

### Recovery Test:
```bash
# Can I restore from GitHub?
git clone https://github.com/markfietje/jetson-openclaw-setup.git

# Can I rebuild services?
cd jetson-openclaw-setup/services/brain-server
cargo build --release

# Can I restore config?
cp workspace/config.yaml ~/.openclaw/workspace/
```

---

## 🎯 Recommended Actions

### Immediate (Today)

1. **Run cleanup script:**
   ```bash
   # Copy script to Jetson
   scp /tmp/cleanup-jetson-workspace.sh jetson:~/.openclaw/workspace/scripts/
   
   # Run on Jetson
   ssh jetson
   chmod +x ~/.openclaw/workspace/scripts/cleanup-jetson-workspace.sh
   ~/.openclaw/workspace/scripts/cleanup-jetson-workspace.sh
   ```

2. **Sync to GitHub:**
   ```bash
   # Copy sync script to MacBook
   cp /tmp/sync-jetson-to-github.sh ~/Sites/jetson-openclaw-setup/scripts/
   
   # Run on MacBook
   chmod +x ~/Sites/jetson-openclaw-setup/scripts/sync-jetson-to-github.sh
   ~/Sites/jetson-openclaw-setup/scripts/sync-jetson-to-github.sh
   ```

3. **Verify everything works:**
   ```bash
   # Check services
   sudo systemctl status brain-server
   sudo systemctl status signal-gateway
   
   # Check OpenClaw
   curl http://127.0.0.1:8765/health
   curl http://127.0.0.1:8080/v1/health
   ```

### Ongoing (Automated)

1. **Add to crontab:**
   ```bash
   # Edit crontab
   crontab -e
   
   # Add daily cleanup at 3 AM
   0 3 * * * ~/.openclaw/workspace/scripts/cleanup-jetson-workspace.sh
   ```

2. **Weekly sync:**
   ```bash
   # Add to HEARTBEAT.md as weekly task
   ```

---

## 💡 Best Practices

1. **Git = Source of Truth**
   - Everything in GitHub is backed up
   - Jetson is just a working copy
   - Can always restore from GitHub

2. **Clean Workspace = Happy Developer**
   - Remove old files regularly
   - Keep only what you need
   - Don't hoard backups (that's what GitHub is for!)

3. **Automate Everything**
   - Daily cleanup (cron)
   - Weekly sync (manual/script)
   - Monthly review

4. **Test Your Backups**
   - Can you restore from GitHub?
   - Can you rebuild services?
   - Are critical files safe?

---

## 🚨 Recovery Procedures

### If Jetson crashes:
1. Get new Jetson/hardware
2. Clone GitHub repo
3. Run deployment script from release
4. Restore config and memory from GitHub workspace
5. Done!

### If you accidentally delete something:
1. Check GitHub (git reflog)
2. Check backup directory created by cleanup script
3. Check daily backups (if enabled)

### If GitHub is down:
1. Local backup on MacBook still works
2. Jetson continues running independently
3. Push when GitHub is back

---

## 📈 Benefits

**Space Saved:** ~2GB
**Organization:** 10x better
**Peace of Mind:** Everything backed up
**Recovery Time:** <30 minutes (from scratch)
**Version History:** Complete (Git)

---

**Bottom line:** Keep GitHub as your primary backup, Jetson as a clean working copy, and MacBook as development environment. ✅
