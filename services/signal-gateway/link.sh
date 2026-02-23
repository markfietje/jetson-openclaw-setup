#!/bin/bash
CONFIG=${1:-/etc/signal-gateway/config.yaml}
echo 'Starting Signal linking...'
echo ''

# Run in background, capture output
timeout 120 signal-gateway link -c "$CONFIG" --device-name 'OpenClaw-Jetson' 2>&1 | tee /tmp/signal-link.log &
PID=$!

# Wait for URL to appear in output
for i in $(seq 1 10); do
  sleep 0.5
  URL=$(grep -o 'sgnl://linkdevice[^ ]*' /tmp/signal-link.log 2>/dev/null | head -1)
  if [ -n "$URL" ]; then
    break
  fi
done

if [ -n "$URL" ]; then
  echo ''
  echo '========== SCAN THIS QR CODE WITH SIGNAL =========='
  echo ''
  qrencode -t ANSIUTF8 "$URL"
  echo ''
  echo 'Or manually enter this URL:'
  echo "$URL"
  echo ''
  echo 'Waiting for scan... (press Ctrl+C to cancel)'
fi

# Wait for the linking to complete
wait $PID
