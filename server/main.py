from fastapi import FastAPI, Query, Response, status
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import FileResponse, StreamingResponse
from starlette.background import BackgroundTask
import scraper
import pdf_gen
from scraper import search, get_chapters, proxy_image
from dotenv import load_dotenv

load_dotenv()

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


@app.get("/health")
async def health():
    return {"status": "ok"}


@app.get("/download")
async def download_chapter(
    ids: list = Query(..., description="List of IDs", alias="ids[]"),
    source: str = Query(..., description="Source number"),
    response: Response = Response(status_code=200)
):
    """
    Download archive with selected chapters.
    """
    try:
        zip_path = pdf_gen.get_chapter_images(ids, source)
        if not zip_path:
            response.status_code = status.HTTP_500_INTERNAL_SERVER_ERROR
            return {"error": "Failed to generate zip file"}

        return FileResponse(
            path=zip_path,
            filename="Chapters.zip",
            media_type="application/zip",
            background=BackgroundTask(
                pdf_gen.cleanup, zip_path
            ),  # Cleanup after sending
        )
    except Exception as e:
        response.status_code = status.HTTP_500_INTERNAL_SERVER_ERROR
        return {"error": str(e)}


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
        return scraper.proxy_image(url,hd)
    except Exception as e:
        return {"error": str(e)}
