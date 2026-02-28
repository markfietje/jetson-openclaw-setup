#!/bin/bash
# Signal Gateway Wrapper - Ensures receiver starts with retry logic

set -euo pipefail

GATEWAY_PID=""
MAX_RETRIES=5
BASE_DELAY=5
HEALTH_TIMEOUT=30
SIGNAL_RECEIVED=""

log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" >&2
}

# Forward signals to the gateway process
forward_signal() {
    local sig=$1
    SIGNAL_RECEIVED="$sig"
    if [ -n "$GATEWAY_PID" ] && kill -0 "$GATEWAY_PID" 2>/dev/null; then
        log "Forwarding $sig to signal-gateway (PID: $GATEWAY_PID)"
        kill -"$sig" "$GATEWAY_PID" 2>/dev/null || true
    fi
}

# Trap signals and forward them
trap 'forward_signal TERM' SIGTERM
trap 'forward_signal INT' SIGINT

# Start the signal-gateway in background
log "Starting signal-gateway..."
/usr/local/bin/signal-gateway serve -c /etc/signal-gateway/config.yaml &
GATEWAY_PID=$!

# Wait for gateway to be healthy
log "Waiting for gateway to be healthy..."
for i in $(seq 1 "$HEALTH_TIMEOUT"); do
    if [ -n "$SIGNAL_RECEIVED" ]; then
        log "Signal $SIGNAL_RECEIVED received during health check, exiting..."
        exit 1
    fi
    
    if curl -s http://127.0.0.1:8080/v1/health >/dev/null 2>&1; then
        log "Gateway is healthy!"
        break
    fi
    
    if [ "$i" -eq "$HEALTH_TIMEOUT" ]; then
        log "ERROR: Gateway failed to become healthy after ${HEALTH_TIMEOUT}s"
        exit 1
    fi
    sleep 1
done

# Wait for brain-server to be available
log "Waiting for brain-server to be available..."
BRAIN_SERVER_URL="${BRAIN_SERVER_URL:-http://127.0.0.1:8765}"
BRAIN_TIMEOUT=30
for i in $(seq 1 "$BRAIN_TIMEOUT"); do
    if [ -n "$SIGNAL_RECEIVED" ]; then
        log "Signal $SIGNAL_RECEIVED received, exiting..."
        exit 1
    fi
    
    if curl -s "${BRAIN_SERVER_URL}/health" >/dev/null 2>&1; then
        log "Brain server is available!"
        break
    fi
    
    if [ "$i" -eq "$BRAIN_TIMEOUT" ]; then
        log "WARNING: Brain server not available after ${BRAIN_TIMEOUT}s, continuing anyway..."
        break
    fi
    sleep 1
done

# Start receiver with exponential backoff retry
log "Starting Signal receiver..."
RETRY_COUNT=0
DELAY=$BASE_DELAY

while [ $RETRY_COUNT -lt $MAX_RETRIES ]; do
    if [ -n "$SIGNAL_RECEIVED" ]; then
        log "Signal $SIGNAL_RECEIVED received, exiting..."
        exit 1
    fi
    
    RETRY_COUNT=$((RETRY_COUNT + 1))
    log "Attempt $RETRY_COUNT/$MAX_RETRIES: Starting receiver..."

    RESPONSE=$(curl -s -X POST http://127.0.0.1:8080/api/v1/rpc \
        -H "Content-Type: application/json" \
        -d '{"jsonrpc":"2.0","id":1,"method":"startReceiver","params":{}}' 2>/dev/null || echo '{"error":"connection failed"}')

    # Check if receiver started successfully
    if echo "$RESPONSE" | grep -q '"result":"Receiver started"'; then
        log "✅ Receiver started successfully!"
        
        # Monitor the gateway process and forward signals
        log "Monitoring signal-gateway (PID: $GATEWAY_PID)..."
        wait "$GATEWAY_PID"
        EXIT_CODE=$?
        
        if [ -n "$SIGNAL_RECEIVED" ]; then
            log "Received $SIGNAL_RECEIVED, gateway exited with code $EXIT_CODE"
        else
            log "Gateway exited with code $EXIT_CODE"
        fi
        
        exit $EXIT_CODE
    fi

    # Check for error in response
    ERROR=$(echo "$RESPONSE" | jq -r '.error // "unknown"' 2>/dev/null || echo "parse error")
    log "❌ Attempt $RETRY_COUNT failed: $ERROR"

    if [ $RETRY_COUNT -lt $MAX_RETRIES ]; then
        log "Retrying in ${DELAY}s..."
        sleep "$DELAY"
        DELAY=$((DELAY * 2))  # Exponential backoff
    fi
done

# All retries failed
log "❌ ERROR: Failed to start receiver after $MAX_RETRIES attempts"
log "This may indicate a network connectivity issue or Signal service outage"
exit 1
