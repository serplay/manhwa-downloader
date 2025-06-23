import os
import re
import requests as req
import shutil
import uuid
from zipfile import ZipFile
from typing import Callable, Optional
from Utils.cleanup import cleanup
from dotenv import load_dotenv
from seleniumbase import SB
from Formats.pdf import gen_pdf
from Formats.cbr import gen_cbr
from Formats.cbz import gen_cbz
from Formats.epub import gen_epub
from Manga.Bato import Bato
from Manga.Asurascans import Asura
from Manga.Manhuaus import Manhuaus

load_dotenv()

MANGAPI_URL = os.environ.get("MANGAPI_URL")


def get_chapter_images(ids, source, progress_callback: Optional[Callable] = None):
    """
    Download chapter images and generate PDF/ZIP files.
    
    Args:
        ids: List of chapter IDs
        source: Source number
        progress_callback: Callback function for progress updates (optional)
    """
    try:
        source = int(source)
    except ValueError:
        raise ValueError(f"Invalid source: {source}. Please choose a valid source.")
    
    total_chapters = len(ids)
    
    def update_progress(current: int, status: str = "Processing..."):
        """Update progress if callback is available"""
        if progress_callback:
            progress = int((current / total_chapters) * 100)
            progress_callback(progress, status)
    
    update_progress(0, "Starting download...")
    
    match source:
        case 0:  # MangaDex
            athome_url = "https://api.mangadex.org/at-home/server/"
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
                            response = req.get(f"{athome_url}{chap_id}", timeout=10)
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

        case 1:  # Manhuaus
            return Manhuaus.download_chapters(ids, update_progress)
        
        case 2:  # Yakshascans
            base_url = "https://yakshascans.com/manga/"
            path = f'Downloads/{uuid.uuid4().hex}'
            pdfs = []
            os.makedirs(f"{path}", exist_ok=True)

            try:
                with SB(uc=True, xvfb=True) as sb:
                    for i, chap_id in enumerate(ids):
                        update_progress(i, f"Downloading chapter {i+1}/{total_chapters}")
                        
                        chap_id, chap_num = chap_id.split("_")
                        
                        try:
                            sb.uc_open_with_reconnect(f"{base_url}{chap_id}/", 4)
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
        
        case 3:  # Asurascan
            return Asura.download_chapters(ids, update_progress)

        case 4:  # Kunmanga
            raise NotImplementedError("Downloading chapters from Kunmanga is not implemented yet.")

        case 5:  # Toonily
            raise NotImplementedError("Downloading chapters from Toonily is not implemented yet.")

        case 6:  # Toongod
            raise NotImplementedError("Downloading chapters from Toongod is not implemented yet.")

        case 7:  # Mangahere        
            base_url = f"{MANGAPI_URL}/manga/mangahere/read"
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
                    
                    response = req.get(f"{base_url}", timeout=10, params={"chapterId": chap_id})
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
        
        case 8:  # Mangapill
            base_url = f"{MANGAPI_URL}/manga/mangapill/read"
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
                    
                    response = req.get(f"{base_url}", timeout=10, params={"chapterId": chap_id})
                    response.raise_for_status()
                    data = response.json()
                    
                    image_links = []
                    for page in data:
                        image_links.append((page["img"], "https://mangapill.com"))
                    
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
        
        case 9:  # Bato
            return Bato.download_chapters(ids, update_progress)
            
        case 10:  # Weebcentral
            base_url = "https://weebcentral.com/chapters/"
            path = f'Downloads/{uuid.uuid4().hex}'
            pdfs = []
            os.makedirs(f"{path}", exist_ok=True)

            try:
                with SB(uc=True, xvfb=True) as sb:
                    for i, chap_id in enumerate(ids):
                        update_progress(i, f"Downloading chapter {i+1}/{total_chapters}")
                        
                        chap_id, chap_num = chap_id.split("_")
                        
                        try:
                            sb.uc_open_with_reconnect(f"{base_url}{chap_id}/", 4)
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
        case _:
            raise ValueError(f"Invalid source: {source}. Please choose a valid source.")
