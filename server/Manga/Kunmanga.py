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


class Kunmanga:
    
    BASE_URL = "https://kunmanga.com"
    
    SEARCH_ELEM = ('div',{"class":"c-tabs-item"})
    SEARCH_PARAMS = "&post_type=wp-manga&op=&author=&artist=&release=&adult="
    CHAPTERS_ELEM = (('ul[class="main version-chap no-volumn active"]',{"class":"main version-chap no-volumn active"}),('ul[class="main version-chap no-volumn"]',{"class":"main version-chap no-volumn"}))
    
    @staticmethod
    def search(title: str):
        """
        Search for comics on Kunmanga by title.
        
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
            Exception: If scraping Kunmanga website fails.
        """
        
        try:
            soup = get_with_captcha(f"{Kunmanga.BASE_URL}?s={title}{Kunmanga.SEARCH_PARAMS}", Kunmanga.SEARCH_ELEM[0])
        except Exception as e:
            raise Exception(f"Failed to fetch data from Kunmanga: {e}")
        
        if not soup:
                return {"message": "No results found."}

        data = soup.find_all("div",{"class":"row c-tabs-item__content"})
            
        comics: ComicsDict = {}
        for num,com in enumerate(data):
            link = com.find("div",{"class":"tab-thumb c-image-hover"}).a["href"][27:]
            title = {"en":com.find("div",{"class":"tab-thumb c-image-hover"}).a["title"]}
            cover = com.find("div",{"class":"tab-thumb c-image-hover"}).a.img["src"]
            header = Kunmanga.BASE_URL
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
        Retrieve the list of chapters for a given comic from Kunmanga.

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
            Exception: If scraping Kunmanga website fails.
        """
        
        try:
            soup = get_with_captcha(f"{Kunmanga.BASE_URL}/manga/{id}", Kunmanga.CHAPTERS_ELEM[0][0])
            if type(soup) is dict:
                soup = get_with_captcha(f"{Kunmanga.BASE_URL}/manga/{id}", Kunmanga.CHAPTERS_ELEM[1][0])
                data = soup.find("ul",Kunmanga.CHAPTERS_ELEM[1][1] ).find_all("li")
            else:
                data = soup.find("ul",Kunmanga.CHAPTERS_ELEM[0][1] ).find_all("li")
        except Exception as e:
            raise Exception(f"Failed to fetch data from Kunmanga: {e}")
        
        chapters: ChaptersDict = {}
        volume = "Vol 1"
        chapters[volume] = VolumeData(volume=volume, chapters={})
        for num, chap in enumerate(data):
            chap_data = chap.a
            chap_num =re.sub(r'[\t\r\n]|[Cc]hapter ',"",chap_data.contents[0])
            chap_id = f'{id}/{chap_data["href"].split("/")[-2]}'
            chapters[volume].chapters[str(num)] = ChapterInfo(id=chap_id, chapter=chap_num)
            
        return chapters

    @staticmethod
    def download_chapters(ids, update_progress=None):
        """
        Download chapters from Kunmanga and return the path to the ZIP archive.
        Args:
            ids: List of chapter IDs
            update_progress: Optional callback for progress updates
        Returns:
            Path to the ZIP archive
        """
        
        total_chapters = len(ids)
        path = f'Downloads/{uuid.uuid4().hex}'
        pdfs = []
        os.makedirs(f"{path}", exist_ok=True)
        try:
            with SB(uc=True, xvfb=True) as sb:
                for i, chap_id in enumerate(ids):
                    update_progress(i, f"Downloading chapter {i+1}/{total_chapters}")
                    
                    chap_id, chap_num = chap_id.split("_")
                    
                    try:
                        sb.uc_open_with_reconnect(f"{Kunmanga.BASE_URL}/manga/{chap_id}/", 4)
                        sb.uc_gui_click_captcha()
                        soup = sb.get_beautiful_soup()
                    except Exception as e:
                        print(f"Skipping chapter {chap_num} due to bot evasion: {e}")
                        continue
                    read_container = soup.find("div", {"class": "reading-content"})
                    if not read_container:
                        print(f"Skipping chapter {chap_num} - reading-content not found.")
                        continue
                    images = read_container.find_all("img")
                    image_links = [
                        re.sub(r'[\t\r\n]', "", img.get("data-src", "")) for img in images if img.get("data-src")
                    ]
                    if not image_links:
                        # If no data-src, try src
                        image_links = [
                            re.sub(r'[\t\r\n]', "", img.get("src", "")) for img in images if img.get("src")
                        ]
                    if not image_links:
                        print(f"No images found for chapter {chap_num}. Skipping.")
                        continue
                    
                    pdfs.append(gen_pdf(image_links, chap_num, path))
                    shutil.rmtree(f"{path}/{chap_num}")
            if not pdfs:
                raise Exception("No PDFs were generated, ZIP will not be created.")
            update_progress(total_chapters, "Creating ZIP archive...")
            zip_path = f"{path}/Chapters.zip"
            with ZipFile(zip_path, "w") as chapters_zip:
                for pdf in pdfs:
                    chapters_zip.write(pdf, os.path.basename(pdf))
                    os.remove(pdf)
            
            update_progress(total_chapters, "Finished")
            return zip_path
        except Exception as e:
            cleanup(f'Downloads/{path}')
            raise e