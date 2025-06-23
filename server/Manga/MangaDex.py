import os
import uuid
import requests as req
import shutil
from zipfile import ZipFile
from Formats.pdf import gen_pdf
from Utils.cleanup import cleanup
from Manga.BaseTypes import Comic, ChapterInfo, VolumeData, ChaptersDict, ComicsDict

class MangaDex:
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
        Download chapters from MangaDex and return the path to the ZIP archive.
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
                
                chap_id, chap_num = chap_id.split("_")
                ch_path = f"{path}/{chap_num}"
                os.makedirs(ch_path, exist_ok=True)
                for retry in range(max_retries := 3):
                    try:
                        response = req.get(f"{MangaDex.AT_HOME}{chap_id}", timeout=10)
                        response.raise_for_status()
                        data = response.json()
                    except req.RequestException as e:
                        if retry == max_retries - 1:
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
                pdfs.append(gen_pdf(image_links, chap_num, path))
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
            shutil.rmtree(path)
            raise e