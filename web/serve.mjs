#!/usr/bin/env node
// Zero-dependency static server for the web IDE.
//
//   node web/serve.mjs           # http://127.0.0.1:8140
//   node web/serve.mjs 9000      # another port
//
// `application/wasm` is the load-bearing part: without it the browser's
// `WebAssembly.instantiateStreaming` refuses the module (and `file://` cannot
// work at all, so the page must be served over HTTP).

import { createServer } from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = fileURLToPath(new URL('./www', import.meta.url));
const PORT = Number(process.argv[2] ?? process.env.PORT ?? 8140);
const HOST = '127.0.0.1';

const MIME = {
  '.html': 'text/html; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.wasm': 'application/wasm',
  '.svg': 'image/svg+xml',
  '.ico': 'image/x-icon',
  '.txt': 'text/plain; charset=utf-8',
  '.map': 'application/json; charset=utf-8',
};

const server = createServer(async (req, res) => {
  const url = new URL(req.url ?? '/', `http://${req.headers.host ?? HOST}`);
  let pathname = decodeURIComponent(url.pathname);
  if (pathname.endsWith('/')) {
    pathname += 'index.html';
  }
  const file = path.resolve(ROOT, `.${pathname}`);
  // Refuse anything that escapes the document root.
  if (file !== ROOT && !file.startsWith(ROOT + path.sep)) {
    res.writeHead(403).end('forbidden');
    return;
  }
  try {
    const info = await stat(file);
    if (info.isDirectory()) {
      res.writeHead(301, { location: `${pathname}/` }).end();
      return;
    }
    const body = await readFile(file);
    res.writeHead(200, {
      'content-type': MIME[path.extname(file).toLowerCase()] ?? 'application/octet-stream',
      'content-length': body.length,
      'cache-control': 'no-cache',
    });
    res.end(body);
  } catch {
    res.writeHead(404, { 'content-type': 'text/plain; charset=utf-8' });
    res.end(`not found: ${pathname}`);
  }
});

server.listen(PORT, HOST, () => {
  console.log(`dpm-nbe web IDE  ->  http://${HOST}:${PORT}/`);
  console.log(`serving ${ROOT}`);
});
