from bs4 import BeautifulSoup as bs 
import requests as req
import re

def search(title, source):
    try:
        source = int(source)
    except ValueError:
        return {"message": "Invalid source"}
    
    match source:
        case 0: #MangaDex
            base_url = "https://api.mangadex.org"
            r = req.get(f"{base_url}/manga",
                        params={"title": title,
                                "includes[]":["cover_art"]}
                        )
            data = r.json()["data"]
            if data:
                comics = {}
                for num,com in enumerate(data):
                    title = com["attributes"]["title"]
                    com_id = com["id"]
                    rel = com["relationships"]
                    trans = com["attributes"]["availableTranslatedLanguages"]
                    for i in rel:
                        if i["type"] == "cover_art":
                            cover_art = f'https://uploads.mangadex.org/covers/{com_id}/{i["attributes"]["fileName"]}.256.jpg'
                            break
                    comics[num] = {"id":com_id, 
                                   "title":title, 
                                   "cover_art":cover_art, 
                                   "availableLanguages":trans,
                                   }
                return comics
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

def get_chapters(id: str, source: int):
    try:
        source = int(source)
    except ValueError:
        return {"message": "Invalid source"}
    match source:
        case 0: #MangaDex
            base_url = "https://api.mangadex.org"
            r = req.get(f"{base_url}/manga/{id}/aggregate",
                        params={"translatedLanguage[]": ["en"]
                                }
                        )
            data = r.json()["volumes"]
            
            for vol in data:
                for chap in data[vol]["chapters"]:
                    del data[vol]["chapters"][chap]["others"]
                    del data[vol]["chapters"][chap]["count"]
            return data
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

def search_chapters(chapters, keyword):
    return [
        chapter for chapter in chapters
    ]