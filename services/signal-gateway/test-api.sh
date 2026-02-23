#!/bin/bash
# Test signal-gateway API (signal-cli compatibility)

echo "=== signal-gateway API Test Suite ==="
echo ""

BASE_URL="http://127.0.0.1:8080"

echo "1. Testing /api/v1/check (health)..."
curl -s -w "\nStatus: %{http_code}\n" $BASE_URL/api/v1/check
echo ""

echo "2. Testing JSON-RPC: version method..."
curl -s -X POST $BASE_URL/api/v1/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"version","id":1}' | jq '.'
echo ""

echo "3. Testing JSON-RPC: getAccountNumber..."
curl -s -X POST $BASE_URL/api/v1/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"getAccountNumber","id":2}' | jq '.'
echo ""

echo "4. Testing legacy /health endpoint..."
curl -s $BASE_URL/health | jq '.'
echo ""

echo "5. Testing SSE connection (5 seconds)..."
timeout 5 curl -N -s $BASE_URL/api/v1/events || echo "SSE connection test complete"
echo ""

echo "=== Test Suite Complete ==="
