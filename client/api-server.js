import express from 'express';
import cors from 'cors';
import { createProxyMiddleware } from 'http-proxy-middleware';

const app = express();
const PORT = process.env.PORT || 3001;

// Middleware
app.use(cors());
app.use(express.json());

// Backend URL from environment variable or default to localhost
const BACKEND_URL = process.env.VITE_API_URL || 'http://localhost:8000';

// Proxy middleware for all API requests
app.use('/api', createProxyMiddleware({
  target: BACKEND_URL,
  changeOrigin: true,
  pathRewrite: {
    '^/api': '', // remvoes /api from URL before proxyin to backend
  },
  onProxyReq: (proxyReq, req, res) => {
    console.log(`[API Proxy] ${req.method} ${req.path} -> ${BACKEND_URL}${proxyReq.path}`);
  },
  onProxyRes: (proxyRes, req, res) => {
    console.log(`[API Proxy] Response: ${proxyRes.statusCode} for ${req.path}`);
  },
  onError: (err, req, res) => {
    console.error('[API Proxy] Error:', err.message);
    res.status(500).json({ 
      error: 'Proxy error', 
      message: 'Failed to connect to backend server' 
    });
  }
}));

// Health check endpoint
app.get('/health', (req, res) => {
  res.json({ 
    status: 'ok', 
    timestamp: new Date().toISOString(),
    backend: BACKEND_URL 
  });
});

// Start server
app.listen(PORT, () => {
  console.log(`🚀 API Server running on http://localhost:${PORT}`);
  console.log(`📡 Proxying to backend: ${BACKEND_URL}`);
  console.log(`🔗 API routes available at: http://localhost:${PORT}/api/*`);
});

export default app; 