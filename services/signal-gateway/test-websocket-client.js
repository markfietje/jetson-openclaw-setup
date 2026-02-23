// WebSocket Test Client for signal-gateway
// Run with: node test-websocket-client.js

const WebSocket = require('ws');

const WS_URL = 'ws://localhost:8080/v1/receive/ws';

console.log('=== Signal Gateway WebSocket Test Client ===\n');

const ws = new WebSocket(WS_URL);

ws.on('open', () => {
    console.log('✅ Connected to', WS_URL);
    console.log('Commands:');
    console.log('  Type "ping" to test connection');
    console.log('  Type "receive" to get messages');
    console.log('  Type "close" to disconnect\n');
    
    // Send initial ping
    ws.send('ping');
});

ws.on('message', (data) => {
    console.log('📨 Received:', data.toString());
});

ws.on('error', (error) => {
    console.error('❌ Error:', error.message);
});

ws.on('close', () => {
    console.log('🔌 Connection closed');
    process.exit(0);
});

// Handle stdin
process.stdin.setEncoding('utf8');
process.stdin.on('data', (data) => {
    const input = data.trim();
    
    if (input === 'close') {
        ws.close();
    } else if (input === 'ping' || input === 'receive') {
        ws.send(input);
    } else if (input) {
        console.log('⚠️  Unknown command:', input);
        console.log('   Valid commands: ping, receive, close');
    }
});

console.log('Type a command and press Enter...\n');
