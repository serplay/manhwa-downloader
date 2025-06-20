# API Proxy Configuration

This project provides two ways to set up an API proxy between the frontend and backend:

## Option 1: Vite Proxy (Simple Solution)

A proxy is already configured in `vite.config.js`. All requests to `/api/*` will automatically be forwarded to the backend.

### Running:

```bash
npm run dev
```

### Configuration:

Edit `vite.config.js` to change the backend address:

```javascript
server: {
  proxy: {
    '/api': {
      target: 'http://localhost:8000', // change to your backend
      changeOrigin: true,
      rewrite: (path) => path.replace(/^\/api/, '')
    }
  }
}
```

## Option 2: Separate API Server (Advanced Solution)

A separate API server acts as a middleware and offers more features like logging, error handling, etc.

### Installing dependencies:

```bash
npm install
```

### Configuration:

1. Copy `env.example` to `.env`
2. Edit `.env` to set the backend address:

```env
VITE_API_URL=http://localhost:8000
```

### Running:

#### API Server only:

```bash
npm run dev:api
```

#### API Server + Frontend (recommended):

```bash
npm run dev:full
```

#### Frontend only (with Vite proxy):

```bash
npm run dev
```

## API Endpoints

All backend endpoints are available via `/api`:

* `GET /api/search/?title=...&source=...` – search for comics
* `GET /api/chapters/?id=...&source=...` – fetch chapters
* `GET /api/download/?ids[]=...&source=...` – download files

## Health Check

The API server provides a health check endpoint:

* `GET /health` – check server status

## Logs

The API server logs all requests and responses:

```
[API Proxy] GET /api/search -> http://localhost:8000/search
[API Proxy] Response: 200 for /api/search
```

## Error Handling

If the backend is unavailable, the API server will return a 500 error with information about the issue.
