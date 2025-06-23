from fastapi import FastAPI, Query, Response, status, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import FileResponse
from starlette.background import BackgroundTask
from Utils.ProxyImage import proxy_image
import scraper
from dotenv import load_dotenv
import asyncio
import aiohttp
from Queue.tasks import download_chapters, cleanup_task
from Queue.celery_app import celery_app
import os

load_dotenv()

debug = os.getenv("DEBUG", "false").lower() == "true"

app = FastAPI()
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


@app.get("/")
async def root():
    return {"message": "It Works!"}


SOURCE_URLS = {
    "0": "https://api.mangadex.org",
    "1": "https://manhuaus.com",
    "2": "https://yakshascans.com",
    "3": "https://asuracomic.net",
    "4": None,  # Kunmanga - Not implemented
    "5": None,  # Toonily - Not implemented
    "6": None,  # Toongod - Not implemented
    "7": "https://mangahere.cc",
    "8": "https://mangapill.com",
    "9": "https://bato.si",
    "10": "https://weebcentral.com",
}


async def check_url(session, url):
    if not url:
        return "null"
    try:
        async with session.get(url, timeout=5) as response:
            return "ok" if response.status == 200 else "null"
    except Exception:
        return "null"


@app.get("/status")
async def get_status():
    status = {}
    async with aiohttp.ClientSession() as session:
        tasks = [check_url(session, url) for url in SOURCE_URLS.values()]
        results = await asyncio.gather(*tasks)
        for i, res in enumerate(results):
            status[str(i)] = res
    return {"status": status}


@app.post("/download")
async def start_download(
    ids: list = Query(..., description="List of IDs", alias="ids[]"),
    source: str = Query(..., description="Source number"),
    comic_title: str = Query("Chapters", description="Title of the comic"),
):
    """
    Start downloading chapters in the background.
    Returns a task ID to track progress.
    """
    try:
        # Validate input data
        if not ids:
            raise HTTPException(status_code=400, detail="ID list cannot be empty")
        
        if not source:
            raise HTTPException(status_code=400, detail="Source must be specified")
        
        # Start background task
        task = download_chapters.delay(ids, source, comic_title)
        
        return {
            "task_id": task.id,  # Use the actual Celery task ID
            "status": "Task has been added to the queue",
            "message": f"Started downloading {len(ids)} chapters"
        }
        
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Error while starting task: {str(e)}")


@app.get("/download/status/{task_id}")
async def get_download_status(task_id: str):
    """
    Get download task status.
    """
    try:
        # Check task status directly from Celery
        from Queue.celery_app import celery_app
        
        # Check task status using AsyncResult
        result = celery_app.AsyncResult(task_id)
        
        if debug:
            print(f"DEBUG: Checking status of task {task_id}")
            print(f"DEBUG: Task state: {result.state}")
            print(f"DEBUG: Task info: {result.info}")
        
        if result.state == "PENDING":
            return {
                "task_id": task_id,
                "state": "PENDING",
                "status": "Task is waiting in the queue..."
            }
        elif result.state == "PROGRESS":
            info = result.info or {}
            return {
                "task_id": task_id,
                "state": "PROGRESS",
                "status": info.get("status", "Processing..."),
                "progress": info.get("progress", 0),
                "total_chapters": info.get("total_chapters", 0)
            }
        elif result.state == "SUCCESS":
            info = result.info or {}
            return {
                "task_id": task_id,
                "state": "SUCCESS",
                "status": "Completed",
                "zip_path": info.get("zip_path"),
                "file_size": info.get("file_size"),
                "total_chapters": info.get("total_chapters"),
                "comic_title": info.get("comic_title")
            }
        elif result.state == "FAILURE":
            info = result.info or {}
            return {
                "task_id": task_id,
                "state": "FAILURE",
                "status": "Error",
                "error": info.get("error", str(info)),
                "comic_title": info.get("comic_title")
            }
        else:
            return {
                "task_id": task_id,
                "state": result.state,
                "status": "Unknown status"
            }
        
    except Exception as e:
        print(f"ERROR: Error while checking status: {e}")
        raise HTTPException(status_code=500, detail=f"Error while getting status: {str(e)}")


@app.get("/download/file/{task_id}")
async def download_file(task_id: str):
    """
    Download ZIP file after task completion.
    """
    try:
        if debug:
            print(f"DEBUG: Attempting to download file for task {task_id}")
        
        # Check task status directly from Celery
        from Queue.celery_app import celery_app
        
        result = celery_app.AsyncResult(task_id)
        
        if debug:
            print(f"DEBUG: Task state: {result.state}")
            print(f"DEBUG: Task info: {result.info}")
        
        if result.state != "SUCCESS":
            print(f"DEBUG: Task not successfully completed - state: {result.state}" if debug else "")
            raise HTTPException(
                status_code=400, 
                detail=f"Task was not completed successfully. Status: {result.state}"
            )
        
        info = result.info or {}
        zip_path = info.get("zip_path")
        comic_title = info.get("comic_title", "Chapters")
        
        if debug:
            print(f"DEBUG: ZIP file path: {zip_path}")
        
        if not zip_path:
            print(f"DEBUG: No ZIP file path in task info" if debug else "")
            raise HTTPException(status_code=404, detail="File path was not found in task result")
        
        if not os.path.exists(zip_path):
            print(f"DEBUG: File does not exist: {zip_path}" if debug else "")
            raise HTTPException(status_code=404, detail=f"File not found: {zip_path}")
        
        if debug:
            print(f"DEBUG: File exists, size: {os.path.getsize(zip_path)} bytes")
        
        # Return file for download
        return FileResponse(
            path=zip_path,
            filename=f"{comic_title}.zip",
            media_type="application/zip",
            background=BackgroundTask(
                cleanup_task, zip_path=zip_path
            )
        )
        
    except HTTPException:
        raise
    except Exception as e:
        print(f"ERROR: Error while downloading file: {e}")
        raise HTTPException(status_code=500, detail=f"Error while downloading file: {str(e)}")


@app.get("/search/")
async def search_endpoint(
    title: str = Query(..., description="Title of the comic"),
    source: str = Query(..., description="Source of the book"),
):
    """
    Search for a comic.
    """
    try:
        comics = scraper.search(title, source)
        if comics:
            return comics
        return {"message": "No comics found"}
    except Exception as e:
        return {"message": str(e)}


@app.get("/chapters/")
async def chapters_endpoint(
    id: str = Query(..., description="Id of the comic"),
    source: str = Query(..., description="Source of the comic"),
):
    """
    Get chapters of a comic.
    """
    try:
        chapters = scraper.get_chapters(id, source)
        return chapters
    except Exception as e:
        return {"error": str(e)}


@app.get("/proxy-image")
async def proxy_image_endpoint(
    url: str = Query(..., description="URL of the image to proxy"), 
    hd: str = Query(..., description="Header of the image to proxy")
    ):
    """
    Proxy image requests to handle MangaDex cover art.
    """
    try:
        return proxy_image(url,hd)
    except Exception as e:
        return {"error": str(e)}


@app.get("/health")
async def health_check():
    """
    Check application health and Redis/Celery connections.
    """
    try:
        # Check Redis connection
        celery_app.control.inspect().active()
        
        return {
            "status": "healthy",
            "celery": "connected",
            "message": "Application is running correctly"
        }
    except Exception as e:
        return {
            "status": "unhealthy",
            "celery": "disconnected",
            "error": str(e)
        }

@app.get("/tasks/list")
async def list_tasks():
    """
    List all tasks in the system.
    """
    try:
        from Queue.celery_app import celery_app
        
        # Check active tasks
        inspect = celery_app.control.inspect()
        active_tasks = inspect.active()
        reserved_tasks = inspect.reserved()
        
        tasks_info = {
            "active": active_tasks or {},
            "reserved": reserved_tasks or {},
            "total_active": sum(len(tasks) for tasks in (active_tasks or {}).values()),
            "total_reserved": sum(len(tasks) for tasks in (reserved_tasks or {}).values())
        }
        
        return tasks_info
        
    except Exception as e:
        print(f"ERROR: Error while listing tasks: {e}")
        raise HTTPException(status_code=500, detail=f"Error while listing tasks: {str(e)}")


@app.get("/tasks/{task_id}/info")
async def get_task_info(task_id: str):
    """
    Detailed information about a task.
    """
    try:
        from Queue.celery_app import celery_app
        
        result = celery_app.AsyncResult(task_id)
        
        task_info = {
            "task_id": task_id,
            "state": result.state,
            "ready": result.ready(),
            "successful": result.successful(),
            "failed": result.failed(),
            "info": result.info,
            "result": result.result,
            "traceback": result.traceback
        }
        
        return task_info
        
    except Exception as e:
        print(f"ERROR: Error while fetching task info: {e}")
        raise HTTPException(status_code=500, detail=f"Error while fetching task info: {str(e)}")
