#!/usr/bin/env node
// Minimal fake MCP stdio server for SDK framing tests. Speaks the same
// Content-Length framing as the Rust engine and — critically — replies to
// tools/call with a body containing a multibyte UTF-8 character ("✓"), the
// exact shape of the engine's real `save_context` reply. A byte-vs-char length
// bug in the client parser hangs forever on this reply; a correct parser does not.
let buf = Buffer.alloc(0);
const sep = Buffer.from('\r\n\r\n');

function send(obj) {
  const body = Buffer.from(JSON.stringify(obj), 'utf-8');
  process.stdout.write(`Content-Length: ${body.length}\r\n\r\n`);
  process.stdout.write(body);
}

process.stdin.on('data', (chunk) => {
  buf = Buffer.concat([buf, chunk]);
  while (true) {
    const headerEnd = buf.indexOf(sep);
    if (headerEnd === -1) return;
    const header = buf.subarray(0, headerEnd).toString('utf-8');
    const m = header.match(/Content-Length:\s*(\d+)/i);
    if (!m) { buf = buf.subarray(headerEnd + 4); continue; }
    const len = parseInt(m[1], 10);
    const start = headerEnd + 4;
    if (buf.length < start + len) return;
    const msg = JSON.parse(buf.subarray(start, start + len).toString('utf-8'));
    buf = buf.subarray(start + len);
    if (msg.method === 'initialize') {
      send({ jsonrpc: '2.0', id: msg.id, result: { protocolVersion: '2024-11-05', capabilities: { tools: {} } } });
    } else if (msg.method === 'tools/call') {
      // Multibyte "✓" makes UTF-8 byte length exceed JS-string length by 2 per char.
      send({ jsonrpc: '2.0', id: msg.id, result: { content: [{ type: 'text', text: '✓ Saved entry: `01KWVHGN6NC3JQX5K02Q752ABC`' }] } });
    } else {
      send({ jsonrpc: '2.0', id: msg.id, result: {} });
    }
  }
});
process.stdin.on('end', () => process.exit(0));
