import os
import uuid
import requests as req
import shutil
from zipfile import ZipFile
from Formats.pdf import gen_pdf
from Utils.cleanup import cleanup
from Manga.BaseTypes import Comic, ChapterInfo, VolumeData, ChaptersDict, ComicsDict

class Bato:
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
        
        try:
            r = req.post(
                f'{Bato.BASE_URL}/ap2/',
                json={
                    "query": Bato.CHAPTERS_QUERY,
                    "variables": {"comicId": id, "operationName": "Chapters"}
                }
            )
            r.raise_for_status()
        except req.RequestException as e:
            raise Exception(f"Failed to fetch data from Bato API: {e}")
        
        data = r.json()["data"]["get_comic_chapterList"]
        if not data:
            raise Exception("No chapters found for this comic.")
        
        chapters: ChaptersDict = {}
        for num, chap in enumerate(data):
            if chap["data"]["volume"] is not None:
                volume = f"Vol {chap['data']['volume']}"
            else:
                volume = "Vol 1"
        
            chapter_num = str(chap["data"]["serial"])
            chapter_id = str(chap["data"]["id"])
            
            if volume not in chapters:
                chapters[volume] = VolumeData(volume=volume, chapters={})
        
            chapters[volume].chapters[str(num)] = ChapterInfo(id=chapter_id, chapter=chapter_num)
        
        return chapters

    @staticmethod
    def download_chapters(ids, update_progress=None):
        """
        Download chapters from Bato and return the path to the ZIP archive.
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
                if update_progress:
                    update_progress(i, f"Downloading chapter {i+1}/{total_chapters}")
                temp = chap_id.split("_")
                if len(temp) != 2:
                    chap_id, chap_num = "_".join(temp[:-1]), temp[-1]
                else:
                    chap_id, chap_num = temp
                ch_path = f"{path}/{chap_num}"
                os.makedirs(ch_path, exist_ok=True)
                r = req.post(f'{Bato.BASE_URL}/ap2/', json={"query": Bato.IMAGES_QUERY, "variables": {"getChapterNodeId": chap_id, "operationName": "Images"}})
                r.raise_for_status()
                image_links = r.json()["data"]["get_chapterNode"]["data"]["imageFile"]["urlList"]
                pdfs.append(gen_pdf(image_links, chap_num, path))
                shutil.rmtree(ch_path)
            if not pdfs:
                raise Exception("No PDFs were generated, ZIP will not be created.")
            if update_progress:
                update_progress(total_chapters, "Creating ZIP archive...")
                
            zip_path = f"{path}/Chapters.zip"
            with ZipFile(zip_path, "w") as chapters_zip:
                for pdf in pdfs:
                    chapters_zip.write(pdf, os.path.basename(pdf))
                    os.remove(pdf)
            
            if update_progress:
                update_progress(total_chapters, "Finished")
            return zip_path
        
        except Exception as e:
            cleanup(f'Downloads/{path}')
            raise e

    