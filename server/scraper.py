from bs4 import BeautifulSoup as bs
from requests.utils import quote as req_quote
import requests as req
import re
from fastapi import FastAPI, Response
from fastapi.responses import StreamingResponse
import io
import os
from dotenv import load_dotenv

load_dotenv()

ROOT_URL = os.environ.get("ROOT_URL")
MANGAPI_URL = os.environ.get("MANGAPI_URL")

def proxy_image(url: str, header: str = None):
    if header:
        header = {"Referer": header}
    try:
        response = req.get(url,headers=header, stream=True)
        if response.status_code == 200:
            return StreamingResponse(
                io.BytesIO(response.content),
                media_type=response.headers.get('content-type', 'image/jpeg')
            )
        return None
    except Exception as e:
        print(f"Error proxying image: {e}")
        return None

def search(title, source):
    try:
        source = int(source)
    except:
        raise ValueError(f"Invalid source: {source}. Please choose a valid source.")
    
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
                            cover_art = f'{ROOT_URL}/proxy-image?url=https://uploads.mangadex.org/covers/{com_id}/{i["attributes"]["fileName"]}.256.jpg&hd='
                            break
                    comics[num] = {"id":com_id, 
                                   "title":title, 
                                   "cover_art":cover_art, 
                                   "availableLanguages":trans,
                                   }
                return comics
        case 1: #Manhuaus
            base_url = "https://manhuaus.com"
            r = req.get(base_url,
                        params={
                            "s":title,
                            "post_type":"wp-manga"
                        })
            soup = bs(r.text, "html.parser")
            if "Just a moment" in soup.text or "Please wait while we are checking your browser..." in soup.text:
                return {"message": "Cloudflare challenge failed."}
            comics = {}
            for num,com in enumerate(soup.find_all("div",{"class":"row c-tabs-item__content"})):
                title_and_link = com.find("h3",{"class":"h4"}).find("a")
                title = {"en":title_and_link.text}
                link = title_and_link["href"][27:-1]
                image_cover = com.find("img")["data-src"]
                comics[num] = {"title":title, 
                               "id":link, 
                               "cover_art":image_cover, 
                               "availableLanguages": ["en"], 
                               }
            return comics
        case 2: #Yakshascans
            raise NotImplementedError("Yakshascans is not implemented yet.")
        case 3: #Asurascan
            raise NotImplementedError("Asurascan is not implemented yet.")
        case 4: #Kunmanga
            raise NotImplementedError("Kunmanga is not implemented yet.")
        case 5: #Toonily
            raise NotImplementedError("Toonily is not implemented yet.")
        case 6: #Toongod
            raise NotImplementedError("Toongod is not implemented yet.")
        case 7: #Mangahere
            r = req.get(f"{MANGAPI_URL}/manga/mangahere/{req_quote(title)}")
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
                               "cover_art":f'{ROOT_URL}/proxy-image?url={cover_art}&hd={header}', 
                               "availableLanguages":trans, 
                               }
            return comics
        case 8: #Mangapark
            r = req.get(f"{MANGAPI_URL}/manga/mangapark/{req_quote(title)}")
            data = r.json()["results"]
            comics = {}
            for num, com in enumerate(data):
                com_id = com["id"]
                title = {'en':com["title"]}
                cover_art = com["image"]
                comics[num] = {"id":com_id,
                               "title":title,
                               "cover_art":cover_art,
                               "availableLanguages": ["en"], 
                               }
            return comics
        case 9: #Mangapill
            r = req.get(f"{MANGAPI_URL}/manga/mangapill/{req_quote(title)}")
            data = r.json()
            return
        case 10: #Mangareader
            r = req.get(f"{MANGAPI_URL}/manga/mangahere/{req_quote(title)}")
            data = r.json()
            return
        case 11: #Mangasee123
            r = req.get(f"{MANGAPI_URL}/manga/mangahere/{req_quote(title)}")
            data = r.json()
            return
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
        case 1: #Manhuaus
            base_url = "https://manhuaus.com/manga/"
            r = req.get(f"{base_url}{id}/")
            soup = bs(r.text, "html.parser")
            if "Just a moment" in soup.text or "Please wait while we are checking your browser..." in soup.text:
                return {"message": "Cloudflare challenge failed."}
            chapters = soup.find("ul",{"class":"main version-chap no-volumn"}).find_all("li")
            data = {"Vol 1":{"volume": "Vol 1", "chapters":{}}}
            for i, chap in enumerate(chapters):
                chap_data = chap.a
                chap_num =re.sub(r'[\t\r\n]|[Cc]hapter ',"",chap_data.contents[0])
                chap_id = chap_data["href"].split("/")[-2]
                data["Vol 1"]["chapters"][str(i)] = {"id": f'{id}/{chap_id}', "chapter": chap_num}
            return data
        case 2: #Yakshascans
            raise NotImplementedError("Yakshascans is not implemented yet.")
        case 3: #Asurascan
            raise NotImplementedError("Asurascan is not implemented yet.")
        case 4: #Kunmanga
            raise NotImplementedError("Kunmanga is not implemented yet.")
        case 5: #Toonily
            raise NotImplementedError("Toonily is not implemented yet.")
        case 6: #Toongod
            raise NotImplementedError("Toongod is not implemented yet.")
        case 7: #Mangahere
            r = req.get(f"{MANGAPI_URL}/mangahere/{req_quote(title)}")
            data = r.json()
            print(data)
            return {'message': 'This source is not implemented yet.'}
        case 8: #MangaKakalot
            return
        case 9: #Mangapark
            return
        case 10: #Mangapill
            return
        case 11: #Mangareader
            return
        case 12: #Mangasee123
            return
        case _:
            raise ValueError(f"Invalid source: {source}. Please choose a valid source.")
    return