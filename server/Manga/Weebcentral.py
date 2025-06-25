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


class Weeb:
    """
    Manga source for https://weebcentral.com

    Provides methods to:
    - Search for manga titles.
    - Retrieve chapter lists for a given manga.
    - Download chapter images into directories (one directory per chapter).

    All methods handle site-specific scraping and error handling.
    """
    
    BASE_URL = "https://weebcentral.com"
    
    SEARCH_ELEM = ('article',{"class":"bg-base-300 flex gap-4 p-4"})
    SEARCH_PARAMS = "&sort=Best+Match&order=Descending&official=Any&anime=Any&adult=Any&display_mode=Full+Display"
    
    @staticmethod
    def search(title: str):
        """
        Search for comics on Weebcentral by title.
        
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
            Exception: If scraping Weebcentral website fails.
        """
        
        try:
            soup = get_with_captcha(f"{Weeb.BASE_URL}/search?text={title}{Weeb.SEARCH_PARAMS}", Weeb.SEARCH_ELEM[0])
        except Exception as e:
            raise Exception(f"Failed to fetch data from Weeb: {e}")
        
        if not soup:
                return {"message": "No results found."}
            
        data = soup.find_all("article", Weeb.SEARCH_ELEM[1])
            
        comics: ComicsDict = {}
        for num,com in enumerate(data):
            title_and_link_and_cover = com.find("section", {"class":"w-full lg:w-[25%] xl:w-[20%]"}).a
            link = title_and_link_and_cover["href"][30:]
            cover_and_title = title_and_link_and_cover.find_all("article")
            cover = cover_and_title[0].picture.img["src"]
            title = {"en": cover_and_title[1].find("div", {"class":"text-ellipsis truncate text-white text-center text-lg z-20 w-[90%]"}).contents[0]}
            trans = ["en"]
            comics[num] = Comic(
                id=link,
                title=title,
                cover_art=cover,
                availableLanguages=trans
            )
            
        return comics
    
    @staticmethod
    def get_chapters(id):
        """
        Retrieve the list of chapters for a given comic from Weebcentral.

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
            Exception: If scraping Weebcentral website fails.
        """
        
        try:
            soup = get_with_captcha(f'{Weeb.BASE_URL}/series{id}', 'div[id="chapter-list"]', click=True)
        except Exception as e:
            raise Exception(f"Failed to fetch data from Weebcentral: {e}")
        
        chapters: ChaptersDict = {}
        volume = "Vol 1"
        chapters[volume] = VolumeData(volume=volume, chapters={})
        for num, chap in enumerate(soup.find("div", {"id": "chapter-list"}).find_all("div", {"class": "flex items-center"})):
            chap_id = chap.a["href"].split("/")[-1]
            chap_num = chap.find("span", {'class':"grow flex items-center gap-2"}).span.contents[0]
            chap_num = re.search(r'\d+\.\d+|\d+', chap_num).group()
            chapters[volume].chapters[str(num)] = ChapterInfo(id=chap_id, chapter=chap_num)
            
        return chapters

    @staticmethod
    def download_chapters(ids, update_progress=None):
        """
        Download selected chapters and save all images for each chapter in a separate directory.

        Args:
            ids (list of str): List of chapter identifiers. Each identifier should be in the format required by the source.
            update_progress (callable, optional): Callback function for reporting progress.

        Returns:
            str: Path to the main directory containing subdirectories for each downloaded chapter. Each subdirectory contains all images for that chapter.

        Behavior:
            - For each chapter ID, fetches the chapter page and extracts all image URLs.
            - Downloads all images for the chapter into a dedicated subdirectory.
            - Skips chapters for which no images are found or if scraping fails.
            - Handles site-specific anti-bot measures (e.g., Selenium, captchas) as needed.
            - If an error occurs, cleans up the created directories and raises the exception.

        Raises:
            Exception: If a critical error occurs during the download process (e.g., network failure, site structure change).
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
                        sb.uc_open_with_reconnect(f"{Weeb.BASE_URL}/chapters/{chap_id_val}/", 4)
                        sb.uc_gui_click_captcha()
                        soup = sb.get_beautiful_soup()
                    except Exception as e:
                        print(f"Skipping chapter {chap_num} due to bot evasion: {e}")
                        continue
                    read_container = soup.find("section", {"class": "flex-1 flex flex-col pb-4 cursor-pointer gap-4"})
                    if not read_container:
                        print(f"Skipping chapter {chap_num} - read-container not found.")
                        continue
                    image_links = [
                        image["src"] for image in read_container.find_all("img") if image.get("src")
                    ]
                    if not image_links:
                        print(f"No images found for chapter {chap_num}. Skipping.")
                        continue
                    download_chapter_images(image_links, chap_num, path)
            return path
        except Exception as e:
            shutil.rmtree(path, ignore_errors=True)
            raise e