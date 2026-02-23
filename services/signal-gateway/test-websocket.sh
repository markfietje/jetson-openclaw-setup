#!/bin/bash
# WebSocket Test Client for signal-gateway
# Usage: ./test-websocket.sh

echo "=== Signal Gateway WebSocket Test ==="
echo ""

# Test 1: Health check
echo "1. Testing health endpoint..."
curl -s http://localhost:8080/health | jq '.'
echo ""

# Test 2: WebSocket connection (using websocat if available)
echo "2. Testing WebSocket connection..."
echo "   Send 'ping' to receive 'pong'"
echo "   Send 'receive' to get messages"
echo "   Send 'close' to disconnect"
echo ""

if command -v websocat &> /dev/null; then
    echo "Using websocat..."
    websocat ws://localhost:8080/v1/receive/ws
else
    echo "websocat not found."
    echo ""
    echo "Install with: cargo install websocat"
    echo "Or test with wscat: npm install -g wscat"
    echo ""
    echo "Manual test with wscat:"
    echo "  wscat -c ws://localhost:8080/v1/receive/ws"
    echo ""
    echo "Then type:"
    echo "  ping      # Should reply: pong"
    echo "  receive   # Should return messages array"
    echo "  close     # Disconnect"
fi
