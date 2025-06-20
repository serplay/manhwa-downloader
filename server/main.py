from fastapi import FastAPI, Query, Response, status
from fastapi.middleware.cors import CORSMiddleware
from fastapi.responses import FileResponse, StreamingResponse
from starlette.background import BackgroundTask
import scraper
import pdf_gen
from scraper import search, get_chapters, proxy_image
from dotenv import load_dotenv
import requests
import asyncio
import aiohttp

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
