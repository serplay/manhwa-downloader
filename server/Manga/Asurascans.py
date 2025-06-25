import os
import uuid
import shutil
from zipfile import ZipFile
from Formats.pdf import gen_pdf
from Utils.cleanup import cleanup
from Manga.BaseTypes import Comic, ChapterInfo, VolumeData, ChaptersDict, ComicsDict
from Utils.bot_evasion import get_with_captcha
from seleniumbase import SB
import re
from Formats.image_downloader import download_chapter_images


class Asura:
    
    BASE_URL = "https://asuracomic.net"
    
    SEARCH_ELEM = ('div[class="grid grid-cols-2 sm:grid-cols-2 md:grid-cols-5 gap-3 p-4"]', {"class":"grid grid-cols-2 sm:grid-cols-2 md:grid-cols-5 gap-3 p-4"})
    CHAPTERS_ELEM = ('div[class="pl-4 pr-2 pb-4 overflow-y-auto scrollbar-thumb-themecolor scrollbar-track-transparent scrollbar-thin mr-3 max-h-[20rem] space-y-2.5"]', {'class': "pl-4 pr-2 pb-4 overflow-y-auto scrollbar-thumb-themecolor scrollbar-track-transparent scrollbar-thin mr-3 max-h-[20rem] space-y-2.5"})
    
    @staticmethod
    def search(title: str):
        """
        Search for comics on Asurascans by title.
        
        Args:
            title (str): The title or keyword to search for.

        Returns:
            dict: A dictionary of search results, where each key is a numeric index and each value is a dict containing:
                - id: Comic ID
                - title: Comic title (dict with language key)
                - cover_art: URL to the cover image
                - availableLanguages: List of available languages
            If no results are found, returns a dict with a 'message' key.

        Raises:
            Exception: If scraping Asurascans website fails.
        """
        
        try:
            soup = get_with_captcha(f"{Asura.BASE_URL}/series?page=1&name={title}", Asura.SEARCH_ELEM[0])
        except Exception as e:
            raise Exception(f"Failed to fetch data from Asurascan: {e}")
        
        if not soup:
                return {"message": "No results found."}
            
        comics: ComicsDict = {}
        for num,com in enumerate(soup.find("div",Asura.SEARCH_ELEM[1]).find_all("a")):
            link = com["href"][6:]
            title = {"en":com.find("span", {"class":"block text-[13.3px] font-bold"}).contents[0]}
            cover_art = com.find("img")["src"]
            trans = ["en"]
            
            comics[num] = Comic(
                id=link,
                title=title,
                cover_art=cover_art,
                availableLanguages=trans
            )
            
        return comics
    
    @staticmethod
    def get_chapters(id):
        """
        Retrieve the list of chapters for a given comic from Asurascans.

        Args:
            id (str): The comic ID to fetch chapters for.

        Returns:
            dict: A dictionary where each key is a volume (e.g., 'Vol 1') and each value is a dict containing:
                - volume: Volume label
                - chapters: Dict of chapters, where each key is a numeric index and each value is a dict with:
                    - id: Chapter ID
                    - chapter: Chapter number/label
            Raises an Exception if no chapters are found.

        Raises:
            Exception: If scraping Asurascans website fails.
        """
        
        try:
            soup = get_with_captcha(f'{Asura.BASE_URL}/series/{id}',Asura.CHAPTERS_ELEM[0])
        except Exception as e:
            raise Exception(f"Failed to fetch data from Asurascan: {e}")
        
        data = soup.find('div', Asura.CHAPTERS_ELEM[1]).find_all('a')
        
        chapters: ChaptersDict = {}
        volume = "Vol 1"
        chapters[volume] = VolumeData(volume=volume, chapters={})
        for num, chap in enumerate(data):
            chap_id = chap['href']
            chap_num = re.sub(r'[\t\r\n]|[Cc]hapter ',"",chap.find("h3").contents[0])
            chapters[volume].chapters[str(num)] = ChapterInfo(id=chap_id, chapter=chap_num)
            
        return chapters

    @staticmethod
    def download_chapters(ids, update_progress=None):
        """
        Download chapters from Asurascans and return the path to the directory with chapter subdirectories containing images.
        """
        total_chapters = len(ids)
        path = f'Downloads/{uuid.uuid4().hex}'
        os.makedirs(path, exist_ok=True)
        try:
            with SB(uc=True, xvfb=True) as sb:
                for i, chap_id in enumerate(ids):
                    if update_progress:
                        update_progress(i, f"Downloading chapter {i+1}/{total_chapters}")
                    chap_id_val, chap_num = chap_id.split("_")
                    try:
                        sb.uc_open_with_reconnect(f"{Asura.BASE_URL}/series/{chap_id_val}/", 4)
                        sb.uc_gui_click_captcha()
                        soup = sb.get_beautiful_soup()
                    except Exception as e:
                        print(f"Skipping chapter {chap_num} due to bot evasion: {e}")
                        continue
                    read_container = soup.find_all("div", {"class": "w-full mx-auto center"})
                    if not read_container:
                        print(f"Skipping chapter {chap_num} - read-container not found.")
                        continue
                    image_links = [
                        container.img["src"] for container in read_container if container.img and container.img.get("src")
                    ]
                    if not image_links:
                        print(f"No images found for chapter {chap_num}. Skipping.")
                        continue
                    download_chapter_images(image_links, chap_num, path)
            return path
        except Exception as e:
            shutil.rmtree(path, ignore_errors=True)
            raise e