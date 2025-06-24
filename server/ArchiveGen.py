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

load_dotenv()

MANGAPI_URL = os.environ.get("MANGAPI_URL")


def get_chapter_images(ids, source, progress_callback: Optional[Callable] = None):
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
    
    total_chapters = len(ids)
    
    def update_progress(current: int, status: str = "Processing..."):
        """Update progress if callback is available"""
        if progress_callback:
            progress = int((current / total_chapters) * 100)
            progress_callback(progress, status)
    
    update_progress(0, "Starting download...")
    
    match source:
        case 0:  # MangaDex
            return MangaDex.download_chapters(ids, update_progress)

        case 1:  # Manhuaus
            return Manhuaus.download_chapters(ids, update_progress)
        
        case 2:  # Yakshascans
            return Yaksha.download_chapters(ids, update_progress)
        
        case 3:  # Asurascan
            return Asura.download_chapters(ids, update_progress)

        case 4:  # Kunmanga
            return Kunmanga.download_chapters(ids, update_progress)

        case 5:  # Toonily
            raise NotImplementedError("Downloading chapters from Toonily is not implemented yet.")

        case 6:  # Toongod
            raise NotImplementedError("Downloading chapters from Toongod is not implemented yet.")

        case 7:  # Mangahere
            return Mangahere.download_chapters(ids, update_progress)
        
        case 8:  # Mangapill
            return Mangapill.download_chapters(ids, update_progress)
        
        case 9:  # Bato
            return Bato.download_chapters(ids, update_progress)
            
        case 10:  # Weebcentral
            return Weeb.download_chapters(ids, update_progress)

        case _:
            raise ValueError(f"Invalid source: {source}. Please choose a valid source.")
