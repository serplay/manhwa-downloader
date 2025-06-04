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
        case 1: #Manhuaus
            print(f"Searching for {title} in Manhuaus...")
            base_url = "https://manhuaus.com"
            r = req.get(base_url,
                        params={
                            "s":title,
                            "post_type":"wp-manga"
                        })
            soup = bs(r.text, "html.parser")
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
        case 1: #Manhuaus
            base_url = "https://manhuaus.com/manga/"
            r = req.get(f"{base_url}{id}/")
            soup = bs(r.text, "html.parser")
            max_chap = re.sub(r'[\t\r\n]','',soup.find("ul",{"class":"main version-chap no-volumn"}).find("li").contents[1].contents[0])
            max_chap_num = int(re.search(r'\d+', max_chap).group())
            chapters = {"Vol 1":{"volume": "Vol 1", "count":f"{max_chap_num}", "chapters":{}}}
            for i in range(max_chap_num+1):
                chapters["Vol 1"]["chapters"][str(i)] = {"id": f"chapter-{i}", "chapter": f'{i}'}
            return chapters
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