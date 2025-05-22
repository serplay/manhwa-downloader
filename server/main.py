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

@app.get("/download/")
async def download_chapter(url: str = Query(..., description="URL of the book"), 
                           chapter: str = Query(..., description="Chapter name"),
                           ):
    """
    Download archive with selected chapters.
    """
    pdf_path = pdf_gen.generate_pdf(url, chapter)
    return {"pdf_path": pdf_path}

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
async def get_chapters(url: str = Query(..., description="URL of the comic"), 
                       source: str = Query(..., description="Source of the comic"),
                       ):
    """
    Get chapters of a comic.
    """
    chapters = scraper.get_chapters(url, source)
    return {"chapters": chapters}