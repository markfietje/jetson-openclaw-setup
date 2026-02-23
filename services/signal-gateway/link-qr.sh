#!/bin/bash
echo 'Starting Signal linking... Have your phone ready!'
echo ''

# Remove old log
rm -f /tmp/signal-link.log

# Start linking in background with unbuffered output
stdbuf -oL signal-gateway link -c /etc/signal-gateway/config.yaml --device-name 'OpenClaw-Jetson' 2>&1 > /tmp/signal-link.log &
PID=$!

# Wait for URL to appear (poll every 0.2s)
for i in $(seq 1 50); do
  sleep 0.2
  URL=$(grep -o 'sgnl://linkdevice[^ ]*' /tmp/signal-link.log 2>/dev/null | head -1)
  if [ -n "$URL" ]; then
    echo "$URL" | xargs -I{} qrencode -t ANSIUTF8 {}
    echo ''
    echo 'Scan the QR code above now! (expires in ~90 seconds)'
    echo 'URL: '$URL
    break
  fi
  echo -n '.'
done
echo ''

# Wait for linking to complete
wait $PID
