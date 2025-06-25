import os
from typing import Callable, Optional
from dotenv import load_dotenv
from Formats.pdf import gen_pdf
from Formats.cbr import gen_cbr
from Formats.cbz import gen_cbz
from Formats.epub import gen_epub
from Manga.Bato import Bato
from Manga.Asurascans import Asura
from Manga.Manhuaus import Manhuaus
from Manga.Yakshascans import Yaksha
from Manga.Weebcentral import Weeb
from Manga.MangaDex import MangaDex
from Manga.Mangahere import Mangahere
from Manga.Mangapill import Mangapill
from Manga.Kunmanga import Kunmanga
from Manga.Toongod import Toongod
from Manga.Toonily import Toonily

load_dotenv()

MANGAPI_URL = os.environ.get("MANGAPI_URL")


def get_chapter_images(ids, source, progress_callback: Optional[Callable] = None, comic_f="pdf"):
    """
    Download chapter images and generate PDF/ZIP files.
    
    Args:
        ids: List of chapter IDs
        source: Source number
        progress_callback: Callback function for progress updates (optional)
    """
    try:
        source = int(source)
    except ValueError:
        raise ValueError(f"Invalid source: {source}. Please choose a valid source.")
    
    if comic_f in ["cbr", "cbz", "epub"]:
        raise NotImplementedError(f"{comic_f.upper()} format generation is not implemented yet.")
    
    total_chapters = len(ids)
    
    def update_progress(current: int, status: str = "Processing..."):
        """Update progress if callback is available"""
        if progress_callback:
            progress = int((current / total_chapters) * 100)
            progress_callback(progress, status)
    
    update_progress(0, "Starting download...")
    
    match source:
        case 0:  # MangaDex
            path =  MangaDex.download_chapters(ids, update_progress)

        case 1:  # Manhuaus
            path =  Manhuaus.download_chapters(ids, update_progress)
        
        case 2:  # Yakshascans
            path =  Yaksha.download_chapters(ids, update_progress)
        
        case 3:  # Asurascan
            path =  Asura.download_chapters(ids, update_progress)

        case 4:  # Kunmanga
            path =  Kunmanga.download_chapters(ids, update_progress)

        case 5:  # Toonily
            raise NotImplementedError("Downloading chapters from Toonily is not implemented yet.") #retur Toonily.download_chapters(ids, update_progress)

        case 6:  # Toongod
            raise NotImplementedError("Downloading chapters from Toongod is not implemented yet.") #return Toongod.download_chapters(ids, update_progress)

        case 7:  # Mangahere
            path =  Mangahere.download_chapters(ids, update_progress)
        
        case 8:  # Mangapill
            path =  Mangapill.download_chapters(ids, update_progress)
        
        case 9:  # Bato
            path =  Bato.download_chapters(ids, update_progress)
            
        case 10:  # Weebcentral
            path =  Weeb.download_chapters(ids, update_progress)

        case _:
            raise ValueError(f"Invalid source: {source}. Please choose a valid source.")
    
    match comic_f:
        case "pdf":
            return gen_pdf(path)
        case "cbr":
            return gen_cbr(path)
        case "cbz":
            return gen_cbz(path)
        case "epub":
            return gen_epub(path)
        case _:
            return gen_pdf(path)
