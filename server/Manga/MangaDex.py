import os
import uuid
import requests as req
import shutil
from zipfile import ZipFile
from Formats.pdf import gen_pdf
from Utils.cleanup import cleanup
from Manga.BaseTypes import Comic, ChapterInfo, VolumeData, ChaptersDict, ComicsDict
from Formats.image_downloader import download_chapter_images

class MangaDex:
    """
    Manga source for https://mangadex.org

    Provides methods to:
    - Search for manga titles.
    - Retrieve chapter lists for a given manga.
    - Download chapter images into directories (one directory per chapter).

    All methods handle site-specific scraping and error handling.
    """
    
    BASE_URL = "https://api.mangadex.org"
    AT_HOME = "https://api.mangadex.org/at-home/server/"

    @staticmethod
    def search(title: str):
        """
        Search for comics on MangaDex by title.
        
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
            Exception: If the MangaDex API request fails.
        """
        
        try:
            r = req.get(f"{MangaDex.BASE_URL}/manga",
                        params={"title": title,
                                "includes[]":["cover_art"]}
                        )
            r.raise_for_status()
        except req.RequestException as e:
            raise Exception(f"Failed to fetch data from MangaDex: {e}")
        
        data = r.json()["data"]
        if not data:
            return {"message": "No results found."}
        
        comics: ComicsDict = {}
        for num, com in enumerate(data):
            if "amz" in com["attributes"]["links"]:
                print(True)
                continue
            title = com["attributes"]["title"]
            com_id = com["id"]
            rel = com["relationships"]
            trans = com["attributes"]["availableTranslatedLanguages"]
            for i in rel:
                if i["type"] == "cover_art":
                    cover_art = f'/api/proxy-image?url=https://uploads.mangadex.org/covers/{com_id}/{i["attributes"]["fileName"]}.256.jpg&hd='
                    break
            comics[num] = Comic(
                id=com_id,
                title=title,
                cover_art=cover_art,
                availableLanguages=trans
            )
            
        return comics

    @staticmethod
    def get_chapters(id):
        """
        Retrieve the list of chapters for a given comic from MangaDex.

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
            Exception: If the MangaDex API request fails or no chapters are found.
        """
        
        try:
            r = req.get(f"{MangaDex.BASE_URL}/manga/{id}/aggregate",
                        params={"translatedLanguage[]": ["en"]
                                }
                        )
            r.raise_for_status()
        except req.RequestException as e:
            raise Exception(f"Failed to fetch data from MangaDex: {e}")
        
        data = r.json()["volumes"]
        
        new_data: ChaptersDict = {}

        for vol in data:
            new_vol = f"Vol {vol}"

            chapters = {
                chap: ChapterInfo(**{
                    k: v for k, v in data[vol]["chapters"][chap].items() # Freaky magic 
                    if k in ("id", "chapter") 
                })
                for chap in data[vol]["chapters"]
            }

            new_data[new_vol] = VolumeData(
                volume=new_vol,
                chapters=chapters
            )

        return new_data

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
            for i, chap_id in enumerate(ids):
                if update_progress:
                    update_progress(i, f"Downloading chapter {i+1}/{total_chapters}")
                chap_id, chap_num = chap_id.split("_")
                for retry in range(3):
                    try:
                        response = req.get(f"{MangaDex.AT_HOME}{chap_id}", timeout=10)
                        response.raise_for_status()
                        data = response.json()
                    except req.RequestException as e:
                        if retry == 2:
                            raise Exception(f"Failed to retrieve chapter data after multiple attempts: {e}")
                        continue
                    baseUrl = data.get("baseUrl")
                    hash_url = data["chapter"].get("hash")
                    images = data["chapter"].get("data")
                    if baseUrl and hash_url and images:
                        break
                else:
                    raise Exception("Failed to retrieve chapter data after multiple attempts.")
                image_links = [f"{baseUrl}/data/{hash_url}/{image}" for image in images]
                download_chapter_images(image_links, chap_num, path)
            return path
        except Exception as e:
            shutil.rmtree(path, ignore_errors=True)
            raise e