import os
import uuid
import shutil
from Manga.BaseTypes import Comic, ChapterInfo, VolumeData, ChaptersDict, ComicsDict
from Utils.bot_evasion import get_with_captcha, get_cookies
from seleniumbase import SB
import re
from Formats.image_downloader import download_chapter_images


class Toonily:
    """
    Manga source for https://toonily.com

    Provides methods to:
    - Search for manga titles.
    - Retrieve chapter lists for a given manga.
    - Download chapter images into directories (one directory per chapter).

    All methods handle site-specific scraping and error handling.
    """
    
    BASE_URL = "https://toonily.com"
    
    SEARCH_ELEM = ['div[class="page-listing-item"]']
    SEARCH_PARAMS = '?op&author&artist&adult'
    CHAPTERS_ELEM = ''
    
    @staticmethod
    def search(title: str):
        """
        Search for comics on Toonily by title.
        
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
            Exception: If scraping Toonily website fails.
        """
        
        get_cookies(Toonily.BASE_URL)
        try:
            soup = get_with_captcha(f"{Toonily.BASE_URL}/search/{title}{Toonily.SEARCH_PARAMS}", '')
        except Exception as e:
            raise Exception(f"Failed to fetch data from Toonily: {e}")
        
        if not soup:
                return {"message": "No results found."}

        data = soup.find_all('div', class_="item-thumb c-image-hover")
        
        comics: ComicsDict = {}
        for num,com in enumerate(data):
            link = com.a["href"].split("/")[-2]
            title = {"en":com.a["title"]}
            cover = com.a.img["src"]
            header = Toonily.BASE_URL
            trans = ["en"]
            comics[num] = Comic(
                id=link,
                title=title,
                cover_art=f'/api/proxy-image?url={cover}&hd={header}',
                availableLanguages=trans
            )
            
        return comics
    
    @staticmethod
    def get_chapters(id):
        """
        Retrieve the list of chapters for a given comic from Toonily.

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
            Exception: If scraping Toonily website fails.
        """
        
        try:
            soup = get_with_captcha(f"{Toonily.BASE_URL}/serie/{id}", '')
            data = soup.find_all("li", class_="wp-manga-chapter")
        except Exception as e:
            raise Exception(f"Failed to fetch data from Toonily: {e}")
        
        chapters: ChaptersDict = {}
        volume = "Vol 1"
        chapters[volume] = VolumeData(volume=volume, chapters={})
        for num, chap in enumerate(data):
            chap_data = chap.a
            chap_num =re.sub(r'[\t\r\n]|[Cc]hapter ',"",chap_data.contents[0])
            chap_id = f'{id}/{chap_data["href"].split("/")[-2]}'
            chapters[volume].chapters[str(num)] = ChapterInfo(id=chap_id, chapter=chap_num)
            
        return chapters


    # Cloudflare block, so ain't bothering with it for now
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
                        sb.uc_open_with_reconnect(f"{Toonily.BASE_URL}/serie/{chap_id_val}/", 4)
                        sb.uc_gui_click_captcha()
                        soup = sb.get_beautiful_soup()
                    except Exception as e:
                        print(f"Skipping chapter {chap_num} due to bot evasion: {e}")
                        continue
                    read_container = soup.find("div", {"class": "reading-content"})
                    if not read_container:
                        print(f"Skipping chapter {chap_num} - reading-content not found.")
                        continue
                    images = read_container.find_all("div",{"class":"page-break no-gaps"})
                    image_links = [
                        (re.sub(r'[\t\r\n]', "", div.img.get("data-src", "")), f"{Toonily.BASE_URL}/") for div in images if div.img.get("data-src")
                    ]
                    if not image_links:
                        image_links = [
                            (re.sub(r'[\t\r\n]', "", div.img.get("src", "")), f"{Toonily.BASE_URL}/") for div in images if div.img.get("src")
                        ]
                    if not image_links:
                        print(f"No images found for chapter {chap_num}. Skipping.")
                        continue
                    print(image_links)
                    download_chapter_images(image_links, chap_num, path, True)
            return path
        except Exception as e:
            shutil.rmtree(path, ignore_errors=True)
            raise e