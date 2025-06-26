import os
import uuid
import requests as req
import shutil
from zipfile import ZipFile
from Formats.pdf import gen_pdf
from Utils.cleanup import cleanup
from Manga.BaseTypes import Comic, ChapterInfo, VolumeData, ChaptersDict, ComicsDict
from Formats.image_downloader import download_chapter_images

class Bato:
    """
    Manga source for https://bato.si

    Provides methods to:
    - Search for manga titles.
    - Retrieve chapter lists for a given manga.
    - Download chapter images into directories (one directory per chapter).

    All methods handle site-specific scraping and error handling.
    """
    
    BASE_URL = "https://bato.si"
    
    IMAGES_QUERY = '''
        query Images($getChapterNodeId: ID!) {
          get_chapterNode(id: $getChapterNodeId) {
            data {
              imageFile {
                urlList
              }
            }
          }
        }
    '''
    
    SEARCH_QUERY = '''
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
    
    CHAPTERS_QUERY = '''
        query Chapters($comicId: ID!, $start: Int) {
          get_comic_chapterList(comicId: $comicId, start: $start) {
            data {
              id
              volume
              count_images
              serial
              order
            }
          }
        }
    '''

    @staticmethod
    def search(title: str):
        """
        Search for comics on Bato by title.
        
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
            Exception: If the Bato API request fails.
        """
        
        try:
            r = req.post(
                f'{Bato.BASE_URL}/ap2/',
                json={
                    "query": Bato.SEARCH_QUERY,
                    "variables": {"select": {"word": title}, "operationName": "Search"}
                }
            )
            r.raise_for_status()
        except req.RequestException as e:
            raise Exception(f"Failed to fetch data from Bato API: {e}")
        
        data = r.json()["data"]["get_search_comic"]["items"]
        if not data:
            return {"message": "No results found."}
        
        comics: ComicsDict = {}
        for num, com in enumerate(data):
            com_id = com['data']['id']
            title = {'en': com['data']['name']}
            cover_art = Bato.BASE_URL + com['data']['urlCover300']
            trans = ['en']
            
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
        Retrieve the list of chapters for a given comic from Bato.

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
            Exception: If the Bato API request fails or no chapters are found.
        """
        
        start = 1
        last_serial = 1
        chapters: ChaptersDict = {}

        while True:
            try:
                r = req.post(
                    f'{Bato.BASE_URL}/ap2/',
                    json={
                        "query": Bato.CHAPTERS_QUERY,
                        "variables": {"comicId": id, "start": start, "operationName": "Chapters"}
                    }
                )
                r.raise_for_status()
            except req.RequestException as e:
                raise Exception(f"Failed to fetch data from Bato API: {e}")

            data = r.json()["data"]["get_comic_chapterList"]
            if last_serial == data[-1]["data"]["order"]:
                break
            else:
                last_serial = data[-1]["data"]["order"]
            
            for chap in data:
                volume = f"Vol {chap['data']['volume']}" if chap["data"]["volume"] is not None else "Vol 1"
                chapter_num = str(chap["data"]["serial"])
                chapter_id = str(chap["data"]["id"])

                if volume not in chapters:
                    chapters[volume] = VolumeData(volume=volume, chapters={})

                chapters[volume].chapters[chapter_num] = ChapterInfo(id=chapter_id, chapter=chapter_num)

            start = last_serial + 1

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
            for i, chap_id in enumerate(ids):
                if update_progress:
                    update_progress(i, f"Downloading chapter {i+1}/{total_chapters}")
                temp = chap_id.split("_")
                if len(temp) != 2:
                    chap_id_val, chap_num = "_".join(temp[:-1]), temp[-1]
                else:
                    chap_id_val, chap_num = temp
                ch_path = f"{path}/{chap_num}"
                os.makedirs(ch_path, exist_ok=True)
                r = req.post(f'{Bato.BASE_URL}/ap2/', json={"query": Bato.IMAGES_QUERY, "variables": {"getChapterNodeId": chap_id_val, "operationName": "Images"}})
                r.raise_for_status()
                image_links = r.json()["data"]["get_chapterNode"]["data"]["imageFile"]["urlList"]
                download_chapter_images(image_links, chap_num, path)
            return path
        except Exception as e:
            shutil.rmtree(path, ignore_errors=True)
            raise e

    