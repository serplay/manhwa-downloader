from celery import Celery
import os
from dotenv import load_dotenv

load_dotenv()

# Celery configuration
CELERY_BROKER_URL = os.environ.get("CELERY_BROKER_URL", "redis://localhost:6379/0")
CELERY_RESULT_BACKEND = os.environ.get("CELERY_RESULT_BACKEND", "redis://localhost:6379/0")

# Creating a Celery instance
celery_app = Celery(
    "manhwa_downloader",
    broker=CELERY_BROKER_URL,
    backend=CELERY_RESULT_BACKEND,
    include=["Queue.tasks"]
)

# Celery configuration
celery_app.conf.update(
    task_serializer="json",
    accept_content=["json"],
    result_serializer="json",
    timezone="UTC",
    enable_utc=True,
    task_track_started=True,
    task_time_limit=30 * 60,  # 30 minutes task limit
    task_soft_time_limit=25 * 60,  # 25 minutes soft limit
    worker_prefetch_multiplier=1,
    task_acks_late=True,  # Changed to False for better tracking
    worker_max_tasks_per_child=1000,
    result_expires=3600,  # Results expire after 1 hour
    task_ignore_result=False,  # Do not ignore results
)

# Configuration for download tasks
celery_app.conf.task_routes = {
    "Queue.tasks.download_chapters": {"queue": "downloads"},
    "Queue.tasks.cleanup_task": {"queue": "cleanup"},
}