from Queue.celery_app import celery_app
import pdf_gen
import os
import shutil
import logging
from typing import List, Dict, Any
from dotenv import load_dotenv

load_dotenv()
log_level_str = os.getenv("LOG_LEVEL", "INFO").upper()
log_level = getattr(logging, log_level_str, logging.INFO)

# Logging configuration
logging.basicConfig(
    level=log_level,
    filename='app.log',
    filemode='a',
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)


@celery_app.task(bind=True, name="Queue.tasks.download_chapters")
def download_chapters(self, ids: List[str], source: str, comic_title: str = "Chapters") -> Dict[str, Any]:
    """
    Celery task to download chapters in the background.
    
    Args:
        ids: List of chapter IDs to download
        source: Source identifier (number)
        comic_title: Title of the comic
    
    Returns:
        Dict with task status information
    """
    task_id = self.request.id  # Use the actual Celery task ID
    
    def progress_callback(progress: int, status: str):
        """Callback for updating task progress"""
        print(f"DEBUG: Progress callback - {progress}% - {status}")
        self.update_state(
            state="PROGRESS",
            meta={
                "task_id": task_id,
                "status": status,
                "progress": progress,
                "total_chapters": len(ids),
                "comic_title": comic_title
            }
        )
        logger.info(f"Task {task_id}: {progress}% - {status}")
    
    try:
        print(f"DEBUG: Starting task {task_id} for {len(ids)} chapters")
        
        # Update task status
        self.update_state(
            state="PROGRESS",
            meta={
                "task_id": task_id,
                "status": "Starting download...",
                "progress": 0,
                "total_chapters": len(ids),
                "comic_title": comic_title
            }
        )
        
        logger.info(f"Starting download of {len(ids)} chapters from source {source}")
        
        # Test the callback
        progress_callback(10, "Testing callback...")
        
        # Call the download function from pdf_gen with progress callback
        zip_path = pdf_gen.get_chapter_images(ids, source, progress_callback)
        
        print(f"DEBUG: get_chapter_images finished, zip_path: {zip_path}")
        
        if not zip_path:
            raise Exception("Failed to generate ZIP file")
        
        # Check if the ZIP file exists
        if not os.path.exists(zip_path):
            raise Exception("ZIP file was not created")
        
        file_size = os.path.getsize(zip_path)
        
        print(f"DEBUG: File created - {zip_path}, size: {file_size} bytes")
        
        # Update status to finished successfully
        self.update_state(
            state="SUCCESS",
            meta={
                "task_id": task_id,
                "status": "Download completed successfully",
                "progress": 100,
                "zip_path": zip_path,
                "file_size": file_size,
                "total_chapters": len(ids),
                "comic_title": comic_title
            }
        )
        
        logger.info(f"Download completed successfully. File: {zip_path}, Size: {file_size} bytes")
        
        return {
            "task_id": task_id,
            "status": "SUCCESS",
            "zip_path": zip_path,
            "file_size": file_size,
            "total_chapters": len(ids),
            "comic_title": comic_title
        }
        
    except Exception as e:
        print(f"DEBUG: Error in task {task_id}: {e}")
        logger.error(f"Error during download: {str(e)}")
        
        # Update status to failure
        self.update_state(
            state="FAILURE",
            meta={
                "task_id": task_id,
                "status": f"Error: {str(e)}",
                "error": str(e),
                "comic_title": comic_title
            }
        )
        
        return {
            "task_id": task_id,
            "status": "FAILURE",
            "error": str(e),
            "comic_title": comic_title
        }


@celery_app.task(name="Queue.tasks.cleanup_task")
def cleanup_task(zip_path: str) -> Dict[str, Any]:
    """
    Celery task to clean up temporary files.
    
    Args:
        zip_path: Path to the ZIP file to remove
    
    Returns:
        Dict with task status information
    """
    try:
        if os.path.exists(zip_path):
            # Remove the ZIP file
            os.remove(zip_path)
            
            # Remove the temporary directory
            temp_dir = os.path.dirname(zip_path)
            if os.path.exists(temp_dir):
                shutil.rmtree(temp_dir)
            
            logger.info(f"Successfully removed temporary files: {zip_path}")
            
            return {
                "status": "SUCCESS",
                "message": f"Removed file: {zip_path}"
            }
        else:
            logger.warning(f"File does not exist: {zip_path}")
            return {
                "status": "WARNING",
                "message": f"File does not exist: {zip_path}"
            }
            
    except Exception as e:
        logger.error(f"Error during cleanup: {str(e)}")
        return {
            "status": "FAILURE",
            "error": str(e)
        }
