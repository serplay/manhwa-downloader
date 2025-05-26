from fastapi import FastAPI, Query
from fastapi.middleware.cors import CORSMiddleware
import scraper
import pdf_gen

app = FastAPI()
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_methods=["*"],
    allow_headers=["*"],
)

@app.get("/")
async def root():
    return {"message": "Hello World"}

@app.get("/download")
async def download_chapter(ids: list = Query(..., description="List of IDs", alias="ids[]"), 
                           source: str = Query(..., description="Source number"),
                           ):
    """
    Download archive with selected chapters.
    """
    pdf_gen.get_chapter_images(ids, source)
    return {"message": f"Download request received for {len(ids)} chapters", "ids": ids}

@app.get("/search/")
async def search(title: str = Query(..., description="Title of the comic"), 
                 source: str = Query(..., description="Source of the book"), 
                 ):
    """
    Search for a comic.
    """
    comics = scraper.search(title, source)
    if comics:
        return comics
    return {"message": "No comics found"}

@app.get("/chapters/")
async def get_chapters(id: str = Query(..., description="Id of the comic"), 
                       source: str = Query(..., description="Source of the comic"),
                       ):
    """
    Get chapters of a comic.
    """
    chapters = scraper.get_chapters(id, source)
    return chapters