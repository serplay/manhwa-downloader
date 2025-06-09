import img2pdf
import requests as req
from bs4 import BeautifulSoup as bs
import re
import os
import shutil
import uuid
from zipfile import ZipFile
from PIL import Image

MANGAPI_URL = os.environ.get("MANGAPI_URL")


def get_chapter_images(ids, source):
    try:
        source = int(source)
    except ValueError:
        raise ValueError(f"Invalid source: {source}. Please choose a valid source.")
    
    match source:
        case 0:  # MangaDex
            athome_url = "https://api.mangadex.org/at-home/server/"
            path = uuid.uuid4().hex
            pdfs = []
            os.makedirs(f"{path}", exist_ok=True)
            
            try:
                for chap_id in ids:
                    chap_id, chap_num = chap_id.split("_")
                    ch_path = f"{path}/{chap_num}"
                    os.makedirs(ch_path, exist_ok=True)

                    for i in range(max_retries := 3):
                        try:
                            response = req.get(f"{athome_url}{chap_id}", timeout=10)
                            response.raise_for_status()
                            data = response.json()
                        except req.RequestException as e:
                            if i == max_retries - 1:
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

                zip_path = f"{path}/Chapters.zip"
                with ZipFile(zip_path, "w") as chapters_zip:
                    for pdf in pdfs:
                        chapters_zip.write(pdf, os.path.basename(pdf))
                        os.remove(pdf)
                return zip_path

            except Exception as e:
                shutil.rmtree(path)
                raise e

        case 1:  # Manhuaus
            base_url = "https://manhuaus.com/manga/"
            path = uuid.uuid4().hex
            pdfs = []
            os.makedirs(f"{path}", exist_ok=True)

            try:
                for chap_id in ids:
                    chap_id, chap_num = chap_id.split("_")
                    try:
                        r = req.get(f"{base_url}{chap_id}/", timeout=10)
                        r.raise_for_status()
                        soup = bs(r.text, "html.parser")
                    except req.RequestException as e:
                        print(f"Skipping chapter {chap_num} due to network error: {e}")
                        continue

                    if "Just a moment" in soup.text or "Please wait while we are checking your browser..." in soup.text:
                        print(f"Skipping chapter {chap_num} due to Cloudflare block.")
                        continue

                    read_container = soup.find("div", {"class": "read-container"})
                    if not read_container:
                        print(f"Skipping chapter {chap_num} - read-container not found.")
                        continue

                    images = read_container.find_all("img")
                    image_links = [
                        re.sub(r'[\t\r\n]', "", img.get("data-src", "")) for img in images if img.get("data-src")
                    ]

                    if not image_links:
                        print(f"No images found for chapter {chap_num}. Skipping.")
                        continue

                    pdfs.append(gen_pdf(image_links, chap_num, path))
                    shutil.rmtree(f"{path}/{chap_num}")

                if not pdfs:
                    raise Exception("No PDFs were generated, ZIP will not be created.")

                zip_path = f"{path}/Chapters.zip"
                with ZipFile(zip_path, "w") as chapters_zip:
                    for pdf in pdfs:
                        chapters_zip.write(pdf, os.path.basename(pdf))
                        os.remove(pdf)
                return zip_path

            except Exception as e:
                cleanup(path)
                raise e

        case 2:  # Yakshascans
            raise NotImplementedError("Yakshascans is not implemented yet.")
        
        case 3:  # Asurascan
            raise NotImplementedError("Asurascan is not implemented yet.")

        case 4:  # Kunmanga
            raise NotImplementedError("Kunmanga is not implemented yet.")

        case 5:  # Toonily
            raise NotImplementedError("Toonily is not implemented yet.")

        case 6:  # Toongod
            raise NotImplementedError("Toongod is not implemented yet.")

        case _:
            raise ValueError(f"Invalid source: {source}. Please choose a valid source.")


# generate PDF
def gen_pdf(images, chap_num, path):
    image_paths = []
    ch_path = f"{path}/{chap_num}"
    os.makedirs(ch_path, exist_ok=True)

    try:
        for i, img_url in enumerate(images):
            try:
                img_resp = req.get(img_url, timeout=10)
                img_resp.raise_for_status()
                img_data = img_resp.content
            except req.RequestException as e:
                raise Exception(f"Failed to download image {img_url}: {e}")

            extension = img_url.split(".")[-1].split("?")[0]  # handle URLs with query params
            img_path = f"{ch_path}/{i}.{extension}"

            with open(img_path, "wb") as img_file:
                img_file.write(img_data)

            if extension.lower() == "webp":
                try:
                    im = Image.open(img_path).convert("RGB")
                    jpg_path = f'{img_path}.jpg'
                    im.save(jpg_path, "JPEG")
                    os.remove(img_path)
                    image_paths.append(jpg_path)
                except Exception as e:
                    raise Exception(f"Failed to convert WEBP to JPEG: {e}")
            else:
                image_paths.append(img_path)

        title = f"Chapter_{chap_num}"
        pdf_path = f"{path}/{title}.pdf"

        with open(pdf_path, "wb") as f:
            f.write(img2pdf.convert(image_paths))

        return pdf_path

    except Exception as e:
        # Clean up images if something fails
        shutil.rmtree(ch_path, ignore_errors=True)
        raise e


def cleanup(path):
    path = path[:-12] if path.endswith("/Chapters.zip") else path
    if os.path.exists(path):
        shutil.rmtree(path)
    else:
        print(f"The path {path} does not exist.")
