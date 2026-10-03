// Copyright (c) Jack Asher
// SPDX-License-Identifier: MPL-2.0

import { readFile } from 'node:fs/promises';
import type { IncomingMessage, ServerResponse } from 'node:http';
import { resolve } from 'node:path';
import { defineConfig, type Plugin } from 'vite';

const localEphemeris = (): Plugin => {
  const datasetPath = process.env.EPHEMERIS_FILE;
  const serve = async (request: IncomingMessage, response: ServerResponse) => {
    if (request.method !== 'GET' && request.method !== 'HEAD') {
      response.writeHead(405, { Allow: 'GET, HEAD' }).end();
      return;
    }
    if (!datasetPath) {
      response.writeHead(503, { 'Content-Type': 'text/plain' })
        .end('Start the local server with EPHEMERIS_FILE=/path/to/cheb.bin');
      return;
    }
    try {
      const bytes = await readFile(resolve(datasetPath));
      response.writeHead(200, {
        'Content-Type': 'application/octet-stream',
        'Content-Length': bytes.length,
        'Cache-Control': 'no-store',
      });
      response.end(request.method === 'HEAD' ? undefined : bytes);
    } catch {
      response.writeHead(503, { 'Content-Type': 'text/plain' })
        .end('The configured ephemeris file is unavailable.');
    }
  };
  const middleware = (request: IncomingMessage, response: ServerResponse, next: () => void) => {
    if (request.url?.split('?')[0] !== '/ephemeris.bin') { next(); return; }
    void serve(request, response);
  };
  return {
    name: 'local-ephemeris',
    configureServer: server => { server.middlewares.use(middleware); },
    configurePreviewServer: server => { server.middlewares.use(middleware); },
  };
};

export default defineConfig({ plugins: [localEphemeris()] });
