from bs4 import BeautifulSoup as bs 
import requests as req
import re
from fastapi import FastAPI, Response
from fastapi.responses import StreamingResponse
import io
import os
import httpx
import ssl

ROOT_URL = os.environ["ROOT_URL"]

ssl_ctx = ssl.SSLContext(protocol=ssl.PROTOCOL_TLSv1_2)
ssl_ctx.set_alpn_protocols(["h2", "http/1.1"])
ssl_ctx.set_ecdh_curve("prime256v1")
ssl_ctx.set_ciphers(
    "TLS_AES_256_GCM_SHA384:TLS_CHACHA20_POLY1305_SHA256:"
    "TLS_AES_128_GCM_SHA256:ECDHE-ECDSA-AES256-GCM-SHA384:"
    "ECDHE-RSA-AES256-GCM-SHA384:DHE-RSA-AES256-GCM-SHA384:"
    "ECDHE-ECDSA-CHACHA20-POLY1305:ECDHE-RSA-CHACHA20-POLY1305:"
    "DHE-RSA-CHACHA20-POLY1305:ECDHE-ECDSA-AES128-GCM-SHA256:"
    "ECDHE-RSA-AES128-GCM-SHA256:DHE-RSA-AES128-GCM-SHA256:"
    "ECDHE-ECDSA-AES256-SHA384:ECDHE-RSA-AES256-SHA384:"
    "DHE-RSA-AES256-SHA256:ECDHE-ECDSA-AES128-SHA256:"
    "ECDHE-RSA-AES128-SHA256:DHE-RSA-AES128-SHA256:"
    "ECDHE-ECDSA-AES256-SHA:ECDHE-RSA-AES256-SHA:"
    "DHE-RSA-AES256-SHA:ECDHE-ECDSA-AES128-SHA:"
    "ECDHE-RSA-AES128-SHA:DHE-RSA-AES128-SHA:"
    "RSA-PSK-AES256-GCM-SHA384:DHE-PSK-AES256-GCM-SHA384:"
    "RSA-PSK-CHACHA20-POLY1305:DHE-PSK-CHACHA20-POLY1305:"
    "ECDHE-PSK-CHACHA20-POLY1305:AES256-GCM-SHA384:"
    "PSK-AES256-GCM-SHA384:PSK-CHACHA20-POLY1305:"
    "RSA-PSK-AES128-GCM-SHA256:DHE-PSK-AES128-GCM-SHA256:"
    "AES128-GCM-SHA256:PSK-AES128-GCM-SHA256:AES256-SHA256:"
    "AES128-SHA256:ECDHE-PSK-AES256-CBC-SHA384:"
    "ECDHE-PSK-AES256-CBC-SHA:SRP-RSA-AES-256-CBC-SHA:"
    "SRP-AES-256-CBC-SHA:RSA-PSK-AES256-CBC-SHA384:"
    "DHE-PSK-AES256-CBC-SHA384:RSA-PSK-AES256-CBC-SHA:"
    "DHE-PSK-AES256-CBC-SHA:AES256-SHA:PSK-AES256-CBC-SHA384:"
    "PSK-AES256-CBC-SHA:ECDHE-PSK-AES128-CBC-SHA256:ECDHE-PSK-AES128-CBC-SHA:"
    "SRP-RSA-AES-128-CBC-SHA:SRP-AES-128-CBC-SHA:RSA-PSK-AES128-CBC-SHA256:"
    "DHE-PSK-AES128-CBC-SHA256:RSA-PSK-AES128-CBC-SHA:"
    "DHE-PSK-AES128-CBC-SHA:AES128-SHA:PSK-AES128-CBC-SHA256:PSK-AES128-CBC-SHA"
)
headers = {"User-agent": 'Mozilla/5.0 (iPhone; CPU iPhone OS 11_5_1; like Mac OS X) AppleWebKit/534.11 (KHTML, like Gecko) Chrome/50.0.3433.281 Mobile Safari/603.0'}

def proxy_image(url: str):
    try:
        response = req.get(url, stream=True)
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
        if source < 0 or source > 6:
            return {"message": "Invalid source. Please choose a valid source."}
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
                            cover_art = f'{ROOT_URL}/proxy-image?url=https://uploads.mangadex.org/covers/{com_id}/{i["attributes"]["fileName"]}.256.jpg'
                            break
                    comics[num] = {"id":com_id, 
                                   "title":title, 
                                   "cover_art":cover_art, 
                                   "availableLanguages":trans,
                                   }
                return comics
        case 1: #Manhuaus
            base_url = "https://manhuaus.com"
            #r = req.get(base_url,
            #            params={
            #                "s":title,
            #                "post_type":"wp-manga"
            #            })
            client = httpx.Client(http2=True, verify=ssl_ctx)
            r = client.get(f"{base_url}/?s={title}&post_type=wp-manga", headers=headers)
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
    return