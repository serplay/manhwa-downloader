import os
import re
import uuid
import requests as req
import shutil
from zipfile import ZipFile
from Formats.pdf import gen_pdf
from Manga.BaseTypes import Comic, ChapterInfo, VolumeData, ChaptersDict, ComicsDict
from dotenv import load_dotenv
from Utils.cleanup import cleanup
load_dotenv()
MANGAPI_URL = os.environ.get("MANGAPI_URL")

class Mangahere:
    BASE_URL = f"{MANGAPI_URL}/manga/mangahere"

    @staticmethod
    def search(title: str):
        """
        Search for comics on Mangahere by title.
        
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
            Exception: If the Mangahere API request fails.
        """
        
        try:
            r = req.get(f"{Mangahere.BASE_URL}/{title}")
        except req.RequestException as e:
            raise Exception(f"Failed to fetch data from Mangahere API.")
        
        data = r.json()["results"]
        
        comics: ComicsDict = {}
        for num, com in enumerate(data):
            com_id = com["id"]
            title = {'en':com["title"]}
            cover_art = com["image"]
            header = com["headerForImage"]
            trans = ["en"]
            comics[num] = Comic(
                id=com_id,
                title=title,
                cover_art=f'/api/proxy-image?url={cover_art}&hd={header}',
                availableLanguages=trans
            )
            
        return comics

    @staticmethod
    def get_chapters(id):
        """
        Retrieve the list of chapters for a given comic from Mangahere.

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
            Exception: If the Mangahere API request fails or no chapters are found.
        """
        
        try:
            r = req.get(f'{Mangahere.BASE_URL}/info', params={"id": id})
            r.raise_for_status()
        except req.RequestException as e:
            raise Exception(f"Failed to fetch data from Mangahere API: {e}")
        
        data: ChaptersDict = {}
        for num, chap in enumerate(r.json()["chapters"]):
            title_raw = chap["title"]

            if "Vol" in title_raw:
                match = re.search(r'Vol\.(\d+)\s+Ch\.(\d+(?:\.\d+)?)', title_raw)
                if match:
                    volume_num, chapter_num = match.groups()
                else:
                    volume_num, chapter_num = "1", "0"  # Fallback if regex fails
            else:
                volume_num = "1"
                match = re.search(r'\d+(?:\.\d+)?', title_raw)
                chapter_num = match.group(0) if match else "0"

            volume_str = f"Vol {int(volume_num)}" if volume_num.isdigit() else f"Vol {float(volume_num)}"
            chapter_str = str(int(chapter_num)) if chapter_num.isdigit() else str(float(chapter_num))

            chap_id = chap["id"]
            chapter_info = ChapterInfo(id=chap_id, chapter=chapter_str)

            if volume_str not in data:
                data[volume_str] = VolumeData(volume=volume_str, chapters={})

            data[volume_str].chapters[str(num)] = chapter_info

        return data

    @staticmethod
    def download_chapters(ids, update_progress=None):
        """
        Download chapters from Mangahere and return the path to the ZIP archive.
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
            for i, chap_id in enumerate(ids):
                update_progress(i, f"Downloading chapter {i+1}/{total_chapters}")
                
                temp = chap_id.split("_")
                if len(temp) != 2:
                    chap_id, chap_num = "_".join(temp[:-1]), temp[-1]
                else:
                    chap_id, chap_num = temp
                ch_path = f"{path}/{chap_num}"
                os.makedirs(ch_path, exist_ok=True)
                
                response = req.get(f"{Mangahere.BASE_URL}/read", timeout=10, params={"chapterId": chap_id})
                response.raise_for_status()
                data = response.json()
                
                image_links = []
                for page in data:
                    image_links.append((page["img"], page["headerForImage"]["Referer"]))
                
                pdfs.append(gen_pdf(image_links, chap_num, path, referer=True))
                shutil.rmtree(ch_path)
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