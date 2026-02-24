#!/bin/bash
# Ingest GutMindSynergy posts into brain-server
# Created: 2026-02-13

GUTMIND_DIR="/home/jetson/.openclaw/workspace/macbook-sites/gutmindsynergy/posts"
BRAIN_URL="http://127.0.0.1:8765"

echo "🧠 Ingesting GutMindSynergy posts into brain-server..."
echo "📂 Source: $GUTMIND_DIR"
echo ""

# Count total markdown files
TOTAL_FILES=$(find "$GUTMIND_DIR" -name "*.md" | wc -l)
echo "📊 Found $TOTAL_FILES markdown files to ingest"
echo ""

# Counter
INGESTED=0
FAILED=0

# Find and ingest each markdown file
find "$GUTMIND_DIR" -name "*.md" -print0 | while read -d '' -r file; do
    # Skip if file doesn't exist
    [ ! -f "$file" ] && continue
    
    # Get relative path for logging
    REL_PATH="${file#$GUTMIND_DIR/}"
    
    # Read file content (raw markdown)
    CONTENT=$(cat "$file")
    
    # Send to brain-server as raw text/markdown
    RESPONSE=$(curl -s -X POST "$BRAIN_URL/ingest/memory" \
        -H "Content-Type: text/markdown" \
        --data-binary "$CONTENT" \
        2>/dev/null)
    
    # Check response
    if echo "$RESPONSE" | jq -e '.success == true' 2>/dev/null; then
        INGESTED=$((INGESTED + 1))
        echo "✅ [$INGESTED/$TOTAL_FILES] $REL_PATH"
    else
        FAILED=$((FAILED + 1))
        echo "❌ FAILED: $REL_PATH"
        echo "   Response: $RESPONSE"
    fi
done

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 INGESTION COMPLETE"
echo "✅ Successfully ingested: $INGESTED files"
echo "❌ Failed: $FAILED files"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "🧠 Brain-stats:"
curl -s "$BRAIN_URL/stats" | jq '.'
