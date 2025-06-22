#!/usr/bin/env python3
"""
Celery Worker for Manhwa Downloader
Run this file to start the Celery worker for processing background tasks.
"""

import os
import sys
from dotenv import load_dotenv

# Add the server directory to the path
sys.path.append(os.path.dirname(os.path.abspath(__file__)))

load_dotenv()

from Queue.celery_app import celery_app

if __name__ == "__main__":
    # Start the Celery worker
    celery_app.worker_main([
        "worker",
        "--loglevel=info",
        "--concurrency=2",  # Number of worker processes
        "--queues=downloads,cleanup",  # Queues to handle
        "--hostname=manhwa-worker@%h"  # Worker name
    ])
