import beautifulsoup4 as bs4
import requests as req
import re

def search(title, source):
    match source:
        case 0: #MangaDex
            base_url = "https://api.mangadex.org"
            r = req.get(f"{base_url}/manga",
                        params={"title": title}
                        )
            return
        case 1: #Manghuas
            return
        case 2: #Yakshascans
            return
        case 3: #Asurascan
            return
        case 4: #Kunmanga
            return
        case 5: #Toonily
            return
        case 6: #Toongod
            return
    return

def get_chapters(title: str):
    # Example response
    return [
        {"chapter": "Chapter 1", "url": "https://.../ch1"},
        {"chapter": "Chapter 2", "url": "https://.../ch2"},
    ]

def search_chapters(chapters, keyword):
    return [
        chapter for chapter in chapters
    ]