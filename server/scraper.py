from bs4 import BeautifulSoup as bs
from requests.utils import quote as req_quote
import requests as req
import re
from fastapi import FastAPI, Response
from fastapi.responses import StreamingResponse
import io
import os
from dotenv import load_dotenv
from bot_evasion import get_with_captcha

load_dotenv()

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
                            cover_art = f'/api/proxy-image?url=https://uploads.mangadex.org/covers/{com_id}/{i["attributes"]["fileName"]}.256.jpg&hd='
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
                soup = get_with_captcha(f"{base_url}/?s={title}&post_type=wp-manga", '') # 'div[class="row c-tabs-item__content"]'
            except Exception as e:
                raise Exception(f"Failed to fetch data from Manhuaus: {e}")
            
            if not soup:
                return {"message": "No results found."}
            comics = {}
            for num,com in enumerate(soup.find_all("div",{"class":"row c-tabs-item__content"})):
                title_and_link = com.find("h3",{"class":"h4"}).find("a")
                title = {"en":title_and_link.text}
                link = title_and_link["href"][27:-1]
                image_cover = f'/api/proxy-image?url={com.find("img")["data-src"]}&hd={base_url}'
                comics[num] = {"title":title, 
                               "id":link, 
                               "cover_art":image_cover, 
                               "availableLanguages": ["en"], 
                               }
            return comics
        case 2: #Yakshascans
            base_url = f"https://yakshascans.com?s={title}&post_type=wp-manga&op=&author=&artist=&release=&adult="
            
            try:
                soup = get_with_captcha(base_url, 'div[class="row c-tabs-item__content"]')
            except Exception as e:
                raise Exception(f"Failed to fetch data from Yakshascans: {e}")
            
            if not soup:
                return {"message": "No results found."}
            comics = {}
            for num,com in enumerate(soup.find_all("div",{"class":"row c-tabs-item__content"})):
                title_and_link = com.find("h3",{"class":"h4"}).find("a")
                title = {"en":title_and_link.text}
                link = title_and_link["href"][30:-1]
                image_cover = com.find("img")["data-src"]
                header = "https://yakshascans.com"
                comics[num] = {"title":title, 
                               "id":link, 
                               "cover_art":f'/api/proxy-image?url={image_cover}&hd={header}', 
                               "availableLanguages": ["en"], 
                               }
            return comics
        
        case 3: #Asurascan
            try:
                soup = get_with_captcha(f"https://asuracomic.net/series?page=1&name={title}", 'div[class="grid grid-cols-2 sm:grid-cols-2 md:grid-cols-5 gap-3 p-4"]')
            except Exception as e:
                raise Exception(f"Failed to fetch data from Asurascan: {e}")
            
            if not soup:
                return {"message": "No results found."}
            comics = {}
            for num,com in enumerate(soup.find("div",{"class":"grid grid-cols-2 sm:grid-cols-2 md:grid-cols-5 gap-3 p-4"}).find_all("a")):
                link = com["href"][6:]
                title = {"en":com.find("span", {"class":"block text-[13.3px] font-bold"}).contents[0]}
                cover_art = com.find("img")["src"]
                comics[num] = {
                    "title": title,
                    "id": link,
                    "cover_art": cover_art,
                    "availableLanguages": ["en"]
                }
                
            return comics
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
            base_url = "https://bato.si"
            
            body = '''
                query Search($select: Search_Comic_Select) {
                  get_search_comic(select: $select) {
                    items {
                      data {
                        id
                        name
                        urlCover300
                        urlPath
                      }
                    }
                  }
                }
                '''
            try:
                r = req.post(f'{base_url}/ap2/', json={"query": body, "variables": {"select": {"word": title}, "operationName": "Search"}})
                r.raise_for_status()
            except req.RequestException as e:
                raise Exception(f"Failed to fetch data from Bato API: {e}")

            data = r.json()["data"]["get_search_comic"]["items"]
            if not data:
                return {"message": "No results found."}
            
            comics = {}
            for num, com in enumerate(data):
                com_id = com['data']['id']
                title = {'en': com['data']['name']}
                cover_art = base_url + com['data']['urlCover300']
                trans = ['en']
                comics[num] = {
                    "id": com_id,
                    "title": title,
                    "cover_art": cover_art,
                    "availableLanguages":trans,
                }
                
            return comics
                
        case 10: # Weebcentral
            base_url = f"https://weebcentral.com/search?text={title}&sort=Best+Match&order=Descending&official=Any&anime=Any&adult=Any&display_mode=Full+Display"
            
            try:
                soup = get_with_captcha(base_url, 'article')
            except Exception as e:
                raise Exception(f"Failed to fetch data from Weebcentral: {e}")
            if not soup:
                return {"message": "No results found."}
            data = soup.find_all("article", {"class":"bg-base-300 flex gap-4 p-4"})
            comics = {}
            
            for num, com in enumerate(data):
                title_and_link_and_cover = com.find("section", {"class":"w-full lg:w-[25%] xl:w-[20%]"}).a
                link = title_and_link_and_cover["href"][30:]
                cover_and_title = title_and_link_and_cover.find_all("article")
                cover = cover_and_title[0].picture.img["src"]
                title = {"en": cover_and_title[1].find("div", {"class":"text-ellipsis truncate text-white text-center text-lg z-20 w-[90%]"}).contents[0]}
                comics[num] = {
                    "id": link,
                    "title": title,
                    "cover_art": cover,
                    "availableLanguages": ["en"],
                }

            return comics
        
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
                    "volume": vol,
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
                soup = get_with_captcha(f"{base_url}{id}/", 'ul[class="main version-chap no-volumn active"]')
                if type(soup) is dict:
                    soup = get_with_captcha(f"{base_url}{id}/", 'ul[class="main version-chap no-volumn"]')
                    chapters = soup.find("ul",{"class":"main version-chap no-volumn"}).find_all("li")
                else:
                    chapters = soup.find("ul",{"class":"main version-chap no-volumn active"}).find_all("li")
            except Exception as e:
                raise Exception(f"Failed to fetch data from Manhuaus: {e}")
            
            data = {"Vol 1":{"volume": "Vol 1", "chapters":{}}}
            
            for i, chap in enumerate(chapters):
                chap_data = chap.a
                chap_num =re.sub(r'[\t\r\n]|[Cc]hapter ',"",chap_data.contents[0])
                chap_id = chap_data["href"].split("/")[-2]
                data["Vol 1"]["chapters"][str(i)] = {"id": f'{id}/{chap_id}', "chapter": chap_num}
            return data
        case 2:  # Yakshascans
            base_url = "https://yakshascans.com/manga/"
            try:
                soup = get_with_captcha(f"{base_url}{id}/", 'ul[class="main version-chap no-volumn active"]')
            except Exception as e:
                raise Exception(f"Failed to fetch data from Manhuaus: {e}")
            
            chapters = soup.find("ul",{"class":"main version-chap no-volumn active"}).find_all("li")
            data = {"Vol 1":{"volume": "Vol 1", "chapters":{}}}
            
            for i, chap in enumerate(chapters):
                chap_data = chap.a
                chap_num =re.sub(r'[\t\r\n]|[Cc]hapter ',"",chap_data.contents[0])
                chap_id = chap_data["href"].split("/")[-2]
                data["Vol 1"]["chapters"][str(i)] = {"id": f'{id}/{chap_id}', "chapter": chap_num}
            return data
            
        case 3:  # Asurascan
            base_url = f'https://asuracomic.net/series/{id}'
            try:
                soup = get_with_captcha(base_url,'div[class="pl-4 pr-2 pb-4 overflow-y-auto scrollbar-thumb-themecolor scrollbar-track-transparent scrollbar-thin mr-3 max-h-[20rem] space-y-2.5"]')
            except Exception as e:
                raise Exception(f"Failed to fetch data from Asurascan: {e}")
            
            data = soup.find('div', {'class': "pl-4 pr-2 pb-4 overflow-y-auto scrollbar-thumb-themecolor scrollbar-track-transparent scrollbar-thin mr-3 max-h-[20rem] space-y-2.5"}).find_all('a')
            
            chapters = {"Vol 1":{"volume": "Vol 1", "chapters": {}}}
            for num, chap in enumerate(data):
                chap_id = chap['href']
                chap_num = re.sub(r'[\t\r\n]|[Cc]hapter ',"",chap.find("h3").contents[0])
                chapters["Vol 1"]["chapters"][str(num)] = {
                    "id": chap_id,
                    "chapter": chap_num
                }

            return chapters
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
            base_url = "https://bato.si"
            body = '''
                query Chapters($comicId: ID!) {
                  get_comic_chapterList(comicId: $comicId) {
                    data {
                      id
                      volume
                      count_images
                      serial
                    }
                  }
                }
            '''
            
            try:
                r = req.post(f'{base_url}/ap2/', json={"query": body, "variables": {"comicId": id, "operationName": "Chapters"}})
                r.raise_for_status()
            except req.RequestException as e:
                raise Exception(f"Failed to fetch data from Bato API: {e}")

            data = r.json()["data"]["get_comic_chapterList"]
            if not data:
                raise Exception("No chapters found for this comic.")
            
            chapters = {}
            for num, chap in enumerate(data):
                if chap["data"]["volume"] is not None:
                    volume = f"Vol {chap['volume']}"
                else:
                    volume = "Vol 1"
                chapter_num = chap["data"]["serial"]
                chapter_id = chap["data"]["id"]
                if volume not in chapters:
                    chapters[volume] = {"volume": volume, "chapters": {}}
                chapters[volume]["chapters"][str(num)] = {
                    "id": chapter_id,
                    "chapter": chapter_num
                }
            
            return chapters
            
        case 10:  # Weebcentral
            base_url = f"https://weebcentral.com/series{id}"
            
            try:
                soup = get_with_captcha(base_url, 'div[id="chapter-list"]', click=True)
            except Exception as e:
                raise Exception(f"Failed to fetch data from Weebcentral: {e}")
            
            data = {"Vol 1":{"volume": "Vol 1", "chapters":{}}}
            for num, chap in enumerate(soup.find("div", {"id": "chapter-list"}).find_all("div", {"class": "flex items-center"})):
                chap_id = chap.a["href"].split("/")[-1]
                chap_num = chap.find("span", {'class':"grow flex items-center gap-2"}).span.contents[0]
                chap_num = re.search(r'\d+\.\d+|\d+', chap_num).group()
                data["Vol 1"]["chapters"][str(num)] = {"id": chap_id, "chapter": chap_num}
                
            return data
        case _:
            raise ValueError(f"Invalid source: {source}. Please choose a valid source.")
    return