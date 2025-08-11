/**
 * @fileoverview Upload server for development mode
 * 
 * Simple HTTP server that handles JSON file uploads and serves them back.
 * This runs alongside the Docusaurus dev server.
 */

const http = require('http');
const url = require('url');
const path = require('path');

// In-memory storage for uploaded files
const uploadedFiles = new Map();
let fileCounter = 0;

const server = http.createServer((req, res) => {
  // Handle CORS
  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'PUT, POST, GET, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type');

  if (req.method === 'OPTIONS') {
    res.writeHead(200);
    res.end();
    return;
  }

  const parsedUrl = url.parse(req.url, true);
  const pathname = parsedUrl.pathname;

  if (req.method === 'PUT' && pathname === '/upload') {
    handleUpload(req, res);
  } else if (req.method === 'GET' && pathname.startsWith('/file/')) {
    const fileId = pathname.split('/')[2];
    handleFileRetrieve(fileId, res);
  } else {
    res.writeHead(404);
    res.end('Not found');
  }
});

function handleUpload(req, res) {
  let body = '';
  req.on('data', chunk => {
    body += chunk.toString();
  });

  req.on('end', () => {
    try {
      // Validate JSON
      JSON.parse(body);

      // Store the file with a unique ID
      const fileId = `upload_${Date.now()}_${++fileCounter}`;
      uploadedFiles.set(fileId, {
        content: body,
        timestamp: Date.now(),
        size: body.length
      });

      // Clean up old files (keep only last 20 uploads)
      if (uploadedFiles.size > 20) {
        const oldestKey = Array.from(uploadedFiles.keys())[0];
        uploadedFiles.delete(oldestKey);
      }

      const response = {
        success: true,
        fileId,
        redirectUrl: `http://localhost:3000/vis?upload=${fileId}`,
        message: 'File uploaded successfully',
        size: body.length
      };

      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify(response));

      console.log(`📤 Uploaded file ${fileId} (${body.length} bytes)`);
    } catch (error) {
      console.error('Upload error:', error);
      res.writeHead(400);
      res.end(JSON.stringify({ error: 'Invalid JSON content' }));
    }
  });
}

function handleFileRetrieve(fileId, res) {
  const file = uploadedFiles.get(fileId);
  if (!file) {
    res.writeHead(404);
    res.end(JSON.stringify({ error: 'File not found' }));
    return;
  }

  res.writeHead(200, { 'Content-Type': 'application/json' });
  res.end(file.content);
  console.log(`📥 Retrieved file ${fileId} (${file.size} bytes)`);
}

const PORT = process.env.UPLOAD_PORT || 3001;
server.listen(PORT, () => {
  console.log(`🚀 Upload server running on http://localhost:${PORT}`);
  console.log(`   - Upload endpoint: PUT http://localhost:${PORT}/upload`);
  console.log(`   - Retrieve endpoint: GET http://localhost:${PORT}/file/<fileId>`);
});

// Graceful shutdown
process.on('SIGTERM', () => {
  console.log('🛑 Upload server shutting down...');
  server.close();
});
