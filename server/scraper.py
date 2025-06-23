import requests as req
import re
import os
from dotenv import load_dotenv
from Manga.Bato import Bato
from Manga.Asurascans import Asura
from Manga.Manhuaus import Manhuaus
from Manga.Yakshascans import Yaksha
from Manga.Weebcentral import Weeb
from Manga.MangaDex import MangaDex
from Utils.bot_evasion import get_with_captcha

load_dotenv()

MANGAPI_URL = os.environ.get("MANGAPI_URL")

def search(title, source):
    try:
        source = int(source)
    except:
        raise ValueError(f"Invalid source: {source}. Please choose a valid source.")
    
    match source:
        case 0: #MangaDex
            return MangaDex.search(title)

        case 1: #Manhuaus
            return Manhuaus.search(title)

        case 2: #Yakshascans
            return Yaksha.search(title)
        
        case 3: #Asurascan
            return Asura.search(title)
        
        case 4: #Kunmanga
            raise NotImplementedError("Kunmanga is not implemented yet.")
        case 5: #Toonily
            raise NotImplementedError("Toonily is not implemented yet.")
        case 6: #Toongod
            raise NotImplementedError("Toongod is not implemented yet.")
        case 7: #Mangahere
            try:
                r = req.get(f"{MANGAPI_URL}/manga/mangahere/{title}")
            except req.RequestException as e:
                raise Exception(f"Failed to fetch data from Mangahere API.")
            
            data = r.json()["results"]
            comics = {}
            
            for num, com in enumerate(data):
                com_id = com["id"]
                title = {'en':com["title"]}
                cover_art = com["image"]
                header = com["headerForImage"]
                trans = ["en"]
                comics[num] = {"id":com_id, 
                               "title":title, 
                               "cover_art":f'/api/proxy-image?url={cover_art}&hd={header}', 
                               "availableLanguages":trans, 
                               }
            return comics
        case 8: #Mangapill
            base_url = f"{MANGAPI_URL}/manga/mangapill"
            try:
                r = req.get(f'{base_url}/{title}')
                r.raise_for_status()
            except req.RequestException as e:
                raise Exception(f"Failed to fetch data from Mangapill API: {e}")
            
            data = r.json()["results"]
            comics = {}
            
            for num, com in enumerate(data):
                com_id = com["id"]
                title = {'en': com["title"]}
                cover_art = com["image"]
                header = 'https://mangapill.com' # guessed referer first try 😅
                trans = ["en"]
                comics[num] = {
                    "id": com_id,
                    "title": title,
                    "cover_art": f'/api/proxy-image?url={cover_art}&hd={header}',
                    "availableLanguages": trans,
                }
            
            return comics
        case 9: #Bato
            return Bato.search(title)
                
        case 10: # Weebcentral
            return Weeb.search(title)
        
        case _:
            raise ValueError(f"Invalid source: {source}. Please choose a valid source.")
    return

def get_chapters(id: str, source: int):
    try:
        source = int(source)
    except:
        raise ValueError(f"Invalid source: {source}. Please choose a valid source.")
        
    match source:
        case 0: #MangaDex
            return MangaDex.get_chapters(id)

        case 1: #Manhuaus
            return Manhuaus.get_chapters(id)

        case 2:  # Yakshascans
            return Yaksha.get_chapters(id)
            
        case 3:  # Asurascan
            return Asura.get_chapters(id)

        case 4:  # Kunmanga
            raise NotImplementedError("Pulling chapters from Kunmanga is not implemented yet.")
        case 5:  # Toonily
            raise NotImplementedError("Pulling chapters from Toonily is not implemented yet.")
        case 6:  # Toongod
            raise NotImplementedError("Pulling chapters from Toongod is not implemented yet.")
        case 7:  # Mangahere        
            base_url = f"{MANGAPI_URL}/manga/mangahere/info"
            try:
                r = req.get(base_url, params={"id": id})
                r.raise_for_status()
            except req.RequestException as e:
                raise Exception(f"Failed to fetch data from Mangahere API: {e}")
            
            data = {}
            for num, chap in enumerate(r.json()["chapters"]):
                if "Vol" in chap["title"]:
                    volume, title = re.search(r'Vol\.(\d+)\s+Ch\.(\d+(?:\.\d+)?)', chap["title"]).groups() #I hate regex, but its so useful 😵‍💫
                    volume = "Vol " + str(int(volume)) if volume.isdigit() else str(float(volume))
                    title = str(int(title)) if title.isdigit() else str(float(title))
                else:
                    volume = "Vol 1"
                    title = re.search(r'\d+(?:\.\d+)?', chap["title"])[0]
                    title = str(int(title)) if title.isdigit() else str(float(title))

                chap_id = chap["id"]
                if volume not in data:
                    data[volume] = {"volume": volume, "chapters": {}}
                    data[volume]["chapters"][str(num)] = {"id": chap_id, "chapter": title}
                else:
                    data[volume]["chapters"][str(num)] = {"id": chap_id, "chapter": title}

            return data
        case 8:  # Mangapill
            base_url = f"{MANGAPI_URL}/manga/mangapill/info"
            try:
                r = req.get(base_url, params={"id": id})
                r.raise_for_status()
            except req.RequestException as e:
                raise Exception(f"Failed to fetch data from Mangahere API: {e}")
            
            data = {"Vol 1":{ "volume": "Vol 1", "chapters": {}}}
            for num, chap in enumerate(r.json()["chapters"]):
                data["Vol 1"]["chapters"][str(num)] = {
                    "id": chap["id"],
                    "chapter": chap["chapter"]
                }
            
            return data
        case 9:  # Bato
            return Bato.get_chapters(id)
            
        case 10:  # Weebcentral
            return Weeb.get_chapters(id)

        case _:
            raise ValueError(f"Invalid source: {source}. Please choose a valid source.")
        