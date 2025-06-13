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
            try:
                r = req.get(f"{base_url}/manga",
                            params={"title": title,
                                    "includes[]":["cover_art"]}
                            )
                r.raise_for_status()
            except req.RequestException as e:
                raise Exception(f"Failed to fetch data from MangaDex: {e}")
            
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
            try:
                r = req.get(base_url,
                            params={
                                "s":title,
                                "post_type":"wp-manga"
                            })
                r.raise_for_status()
            except req.RequestException as e:
                raise Exception(f"Failed to fetch data from Manhuaus: {e}")
            
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
            try:
                r = req.get(f"{MANGAPI_URL}/manga/mangahere/{req_quote(title)}")
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
                               "cover_art":f'{ROOT_URL}/proxy-image?url={cover_art}&hd={header}', 
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
                header = {'Referer':'https://mangapill.com'} # guessed referer first try 😅
                trans = ["en"]
                comics[num] = {
                    "id": com_id,
                    "title": title,
                    "cover_art": f'{ROOT_URL}/proxy-image?url={cover_art}&hd={header}',
                    "availableLanguages": trans,
                }
            
            return comics
        
        case 9: #Mangareader
            raise NotImplementedError("Mangareader is not implemented yet.")
        case 10: #Mangasee123
            raise NotImplementedError("Mangasee123 is not implemented yet.")
        case _:
            raise ValueError(f"Invalid source: {source}. Please choose a valid source.")
    return

def get_chapters(id: str, source: int):
    try:
        source = int(source)
    except:
        raise ValueError(f"Invalid source: {source}. Please choose a valid source.")
        
    match source:
        
        # response general structure:
        # {
        #     "Vol {volume number}": {
        #         "volume": "{volume number}",
        #         "chapters": {
        #             "{chapter number}": {
        #                 "id": "{chapter id}",
        #                 "chapter": "{chapter number}"
        #             }
        #         }
        #     }
        # }
        
        case 0: #MangaDex
            base_url = "https://api.mangadex.org"
            try:
                r = req.get(f"{base_url}/manga/{id}/aggregate",
                            params={"translatedLanguage[]": ["en"]
                                    }
                            )
                r.raise_for_status()
            except req.RequestException as e:
                raise Exception(f"Failed to fetch data from MangaDex: {e}")
            
            r = req.get(f"{base_url}/manga/{id}/aggregate",
                        params={"translatedLanguage[]": ["en"]
                                }
                        )
            data = r.json()["volumes"]
            
            new_data = {}
            for vol in data:
                new_vol = f"Vol {vol}"
                new_data[new_vol] = {
                    "chapters": {
                        chap: {
                            k: v for k, v in data[vol]["chapters"][chap].items()
                            if k not in ("others", "count")
                        }
                        for chap in data[vol]["chapters"]
                    }
                }
                
            return new_data
        case 1: #Manhuaus
            base_url = "https://manhuaus.com/manga/"
            try:
                r = req.get(f"{base_url}{id}/")
                r.raise_for_status()
            except req.RequestException as e:
                raise Exception(f"Failed to fetch data from Manhuaus: {e}")
            
            soup = bs(r.text, "html.parser")
            if "Just a moment" in soup.text or "Please wait while we are checking your browser..." in soup.text:
                raise Exception("Cloudflare challenge failed.")
            
            chapters = soup.find("ul",{"class":"main version-chap no-volumn"}).find_all("li")
            data = {"Vol 1":{"volume": "Vol 1", "chapters":{}}}
            
            for i, chap in enumerate(chapters):
                chap_data = chap.a
                chap_num =re.sub(r'[\t\r\n]|[Cc]hapter ',"",chap_data.contents[0])
                chap_id = chap_data["href"].split("/")[-2]
                data["Vol 1"]["chapters"][str(i)] = {"id": f'{id}/{chap_id}', "chapter": chap_num}
            return data
        case 2:  # Yakshascans
            raise NotImplementedError("Pulling chapters from Yakshascans is not implemented yet.")
        case 3:  # Asurascan
            raise NotImplementedError("Pulling chapters from Asurascan is not implemented yet.")
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
        case 9:  # Mangareader
            raise NotImplementedError("Pulling chapters from Mangareader is not implemented yet.")
        case 10:  # Mangasee123
            raise NotImplementedError("Pulling chapters from Mangasee123 is not implemented yet.")
        case _:
            raise ValueError(f"Invalid source: {source}. Please choose a valid source.")
    return