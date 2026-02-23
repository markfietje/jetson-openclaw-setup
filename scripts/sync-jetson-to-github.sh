#!/bin/bash
# Sync script: Jetson → GitHub monorepo
# Keeps critical files in sync with GitHub, removes local clutter

set -euo pipefail

GITHUB_REPO="$HOME/Sites/jetson-openclaw-setup"
JETSON_WORKSPACE="$HOME/.openclaw/workspace"

# Files to sync to GitHub (keep in version control)
SYNC_FILES=(
    "MEMORY.md"
    "HEARTBEAT.md"
    "AGENTS.md"
    "IDENTITY.md"
    "SOUL.md"
    "TOOLS.md"
    "config.yaml"
)

# Directories to sync
SYNC_DIRS=(
    "memory"
    ".brain-domains"
)

# Files to keep only on Jetson (local only, never commit)
LOCAL_ONLY=(
    "brain.db"
    "brain.db-shm"
    "brain.db-wal"
    "*.db.backup*"
    "brain-server"
    "brain-server-*"
    "target/"
    ".git/"
)

echo "🔄 Syncing Jetson → GitHub monorepo..."
echo ""

# Ensure GitHub repo exists
if [ ! -d "$GITHUB_REPO" ]; then
    echo "❌ GitHub repo not found: $GITHUB_REPO"
    echo "Please clone it first on your MacBook"
    exit 1
fi

cd "$GITHUB_REPO"

# Create workspace backup directory
mkdir -p workspace/jetson-backup

# Sync files
echo "📄 Syncing files..."
for file in "${SYNC_FILES[@]}"; do
    if [ -f "$JETSON_WORKSPACE/$file" ]; then
        cp "$JETSON_WORKSPACE/$file" "workspace/"
        echo "  ✅ $file"
    else
        echo "  ⚠️  $file (not found)"
    fi
done

# Sync directories
echo ""
echo "📁 Syncing directories..."
for dir in "${SYNC_DIRS[@]}"; do
    if [ -d "$JETSON_WORKSPACE/$dir" ]; then
        cp -r "$JETSON_WORKSPACE/$dir" "workspace/"
        echo "  ✅ $dir/"
    else
        echo "  ⚠️  $dir/ (not found)"
    fi
done

# Create .gitignore for workspace
cat > workspace/.gitignore << 'EOF'
# Build artifacts
target/
*.rlib
*.rmeta
*.o
*.a

# Databases (keep backups, not live DBs)
brain.db
brain.db-shm
brain.db-wal
*.db.backup*

# Binaries
brain-server
brain-server-*
signal-gateway

# Logs
*.log

# Temporary files
*.tmp
*.bak

# Old backups
*-backup-*
*-old

# IDE
.vscode/
.idea/
*.swp
*~

# OS
.DS_Store
Thumbs.db
EOF

# Commit changes
cd "$GITHUB_REPO"
git add workspace/
git status

echo ""
echo "✅ Sync complete!"
echo ""
echo "Next steps:"
echo "  1. Review changes: cd $GITHUB_REPO && git status"
echo "  2. Commit: git commit -m 'chore: Sync workspace from Jetson'"
echo "  3. Push: git push origin main"
echo ""
