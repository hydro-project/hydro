/**
 * @fileoverview Upload endpoint for large JSON files
 * 
 * This provides an HTTP endpoint that accepts JSON file uploads for visualization.
 * It stores the file temporarily and redirects to the visualizer with a file ID.
 */

// In-memory storage for uploaded files (in production, use a proper database/storage)
const uploadedFiles = new Map();
let fileCounter = 0;

/**
 * Handle file upload via HTTP PUT/POST
 */
export async function handleFileUpload(request) {
  try {
    const contentType = request.headers.get('content-type');
    
    let jsonContent;
    if (contentType && contentType.includes('application/json')) {
      jsonContent = await request.text();
    } else {
      // Handle form data or raw body
      jsonContent = await request.text();
    }

    // Validate JSON
    try {
      JSON.parse(jsonContent);
    } catch (e) {
      return new Response('Invalid JSON content', { status: 400 });
    }

    // Store the file with a unique ID
    const fileId = `upload_${Date.now()}_${++fileCounter}`;
    uploadedFiles.set(fileId, {
      content: jsonContent,
      timestamp: Date.now(),
      size: jsonContent.length
    });

    // Clean up old files (keep only last 10 uploads)
    if (uploadedFiles.size > 10) {
      const oldestKey = Array.from(uploadedFiles.keys())[0];
      uploadedFiles.delete(oldestKey);
    }

    // Return the file ID and redirect URL
    const redirectUrl = `/docs/vis?upload=${fileId}`;
    
    return new Response(JSON.stringify({
      success: true,
      fileId,
      redirectUrl,
      message: 'File uploaded successfully'
    }), {
      status: 200,
      headers: {
        'Content-Type': 'application/json',
        'Access-Control-Allow-Origin': '*',
        'Access-Control-Allow-Methods': 'PUT, POST, OPTIONS',
        'Access-Control-Allow-Headers': 'Content-Type'
      }
    });
  } catch (error) {
    console.error('Upload error:', error);
    return new Response('Upload failed', { status: 500 });
  }
}

/**
 * Retrieve uploaded file by ID
 */
export function getUploadedFile(fileId) {
  return uploadedFiles.get(fileId);
}

/**
 * Handle CORS preflight requests
 */
export function handleCors() {
  return new Response(null, {
    status: 200,
    headers: {
      'Access-Control-Allow-Origin': '*',
      'Access-Control-Allow-Methods': 'PUT, POST, OPTIONS',
      'Access-Control-Allow-Headers': 'Content-Type'
    }
  });
}
