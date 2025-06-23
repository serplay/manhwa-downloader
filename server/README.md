# Manhwa Downloader – Task Queue System

## Overview

The application has been refactored to use Celery with Redis as a broker for handling background download tasks. This enables:

* Asynchronous processing of download tasks
* Better scalability and performance
* Real-time tracking of task progress
* Automatic cleanup of temporary files

## Requirements

### Redis

Redis is required as a broker for Celery. Install and run Redis:

```bash
# Ubuntu/Debian
sudo apt-get install redis-server
sudo systemctl start redis-server

# macOS
brew install redis
brew services start redis

# Windows
# Download Redis from https://redis.io/download
```

### Python Dependencies

Install the new dependencies:

```bash
pip install -r requirements.txt
```

## Running the Application

### 1. Start Redis

Make sure Redis is running on the default port `6379`.

### 2. Start the Celery Worker

In the `server` directory terminal:

```bash
python celery_worker.py
```

### 3. Start the FastAPI Server

In another terminal in the `server` directory:

```bash
uvicorn main:app --reload --host 0.0.0.0 --port 8000
```

### 4. Start the Frontend

In the `client` directory:

```bash
npm run dev
```

## Configuration

### Environment Variables

Set the following to your `.env` file:

```env
# Redis
CELERY_BROKER_URL=redis://localhost:6379/0
CELERY_RESULT_BACKEND=redis://localhost:6379/0

# Cons
MANGAPI_URL=<consumet-api-url>
```

> See [example](.env.example) environment file

### Celery Configuration

Main settings in `Queue/celery_app.py`:

* `task_time_limit`: 30 minutes (hard time limit for tasks)
* `task_soft_time_limit`: 25 minutes (soft limit)
* `worker_prefetch_multiplier`: 1 (task prefetching)
* `task_acks_late`: True (acknowledge after execution)

## API Endpoints

### New Endpoints

#### POST `/api/download`

Start downloading chapters in the background.

**Parameters:**

* `ids[]`: List of chapter IDs
* `source`: Source number

**Response:**

```json
{
  "task_id": "uuid",
  "celery_task_id": "celery-task-id",
  "status": "Task has been added to the queue",
  "message": "Started downloading X chapters"
}
```

#### GET `/api/download/status/{task_id}`

Get the status of a task.

**Response:**

```json
{
  "task_id": "uuid",
  "state": "PROGRESS",
  "status": "Processing...",
  "progress": 50,
  "total_chapters": 10
}
```

#### GET `/api/download/file/{task_id}`

Download the ZIP file after the task is complete.

#### GET `/api/health`

Check the health of the application and connections to Redis/Celery.

## Task Management

### Task States

* `PENDING`: Task is waiting in the queue
* `PROGRESS`: Task is being processed
* `SUCCESS`: Task completed successfully
* `FAILURE`: Task failed

### Queues

* `downloads`: Chapter download tasks
* `cleanup`: Temporary file cleanup tasks

### Monitoring

You can monitor tasks using Flower (optional):

```bash
pip install flower
celery -A celery_app flower
```

Then open [http://localhost:5555](http://localhost:5555) in your browser.

## Troubleshooting

### Redis is not running

```bash
# Check Redis status
sudo systemctl status redis-server

# Start Redis
sudo systemctl start redis-server
```

### Celery Worker is not connecting to Redis

Check environment variables in `.env`:

```env
CELERY_BROKER_URL=redis://localhost:6379/0
CELERY_RESULT_BACKEND=redis://localhost:6379/0
```

> See [example](.env.example) environment file

### Tasks are not being processed

1. Check if the worker is running
2. Check the worker logs
3. Check Redis connection

### Temporary files are not being deleted

Cleanup tasks run automatically after file downloads. If they aren’t working, check:

1. Worker logs
2. File permissions
3. Cleanup queue configuration

## Performance

### Optimization

* Increase `--concurrency` for more parallel tasks
* Adjust `task_time_limit` depending on file size
* Use Redis Cluster for high availability

### Monitoring

* Use Flower to monitor tasks
* Check Redis and Celery logs
* Monitor memory and CPU usage

## Security

### Redis

* Change Redis default port
* Set a password for Redis
* Restrict Redis access to localhost only

### Temporary Files

* Files are automatically deleted after download
* Use a dedicated directory for temporary files
* Regularly audit and clean up unused files
