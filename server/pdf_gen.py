import img2pdf
import requests as req
from bs4 import BeautifulSoup as bs
import re
import os
import shutil
import uuid
from zipfile import ZipFile
from PIL import Image


def get_chapter_images(ids, source):
    # Download images, save them temporarily
    try:
        source = int(source)
    except ValueError:
        return None
    match source:
        case 0:  # MangaDex
            athome_url = "https://api.mangadex.org/at-home/server/"
            path = uuid.uuid4().hex
            pdfs = []
            os.makedirs(f"{path}", exist_ok=True)
            for chap_id in ids:
                chap_id, chap_num = chap_id.split("_")
                ch_path = f"{path}/{chap_num}"
                os.makedirs(f"{ch_path}", exist_ok=True)
                data = req.get(f"{athome_url}{chap_id}").json()
                baseUrl = data["baseUrl"]
                hash_url = data["chapter"]["hash"]
                images = data["chapter"]["data"]
                image_links = [f"{baseUrl}/data/{hash_url}/{image}" for image in images]
                pdfs.append(gen_pdf(image_links, chap_num, path))
                shutil.rmtree(ch_path)

            zip_path = f"{path}/Chapters.zip"
            with ZipFile(zip_path, "w") as chapters_zip:
                for pdf in pdfs:
                    chapters_zip.write(pdf, os.path.basename(pdf))
                    os.remove(pdf)
            return zip_path
        case 1:  # Manhuaus
            base_url = "https://manhuaus.com/manga/"
            path = uuid.uuid4().hex
            pdfs = []
            os.makedirs(f"{path}", exist_ok=True)
            for chap_id in ids:
                chap_id, chap_num = chap_id.split("_")
                r = req.get(f"{base_url}{chap_id}/")
                soup = bs(r.text, "html.parser")
                if soup.find("span", {"id":"challenge-error-text"}):
                    return {"error": "Cloudflare challenge failed."}
                images = soup.find("div", {"class": "read-container"}).find_all("img")
                image_links = [re.sub(r'[\t\r\n]',"",img["data-src"]) for img in images]
                pdfs.append(gen_pdf(image_links, chap_num, path))
                shutil.rmtree(f"{path}/{chap_num}")
            return
        case 2:  # Yakshascans
            return
        case 3:  # Asurascan
            return
        case 4:  # Kunmanga
            return
        case 5:  # Toonily
            return
        case 6:  # Toongod
            return


# generate PDF
def gen_pdf(images, chap_num, path):
    image_paths = []
    ch_path = f"{path}/{chap_num}"
    os.makedirs(ch_path, exist_ok=True)
    for i, img_url in enumerate(images):
        extension = img_url.split(".")[-1]
        img_data = req.get(img_url).content
        img_path = f"{ch_path}/{i}.{extension}"
        with open(img_path, "wb") as img_file:
            img_file.write(img_data)
        if extension.lower() == "webp":
            # Convert WEBP to JPEG
            im = Image.open(img_path).convert("RGB")
            im.save(f'{img_path}.jpg', "JPEG") # Save as JPEG
            os.remove(img_path)  # Remove the original image file
            image_paths.append(f'{img_path}.jpg')
        else:
            image_paths.append(img_path)
    title = f"Chapter_{chap_num}"
    pdf_path = f"{path}/{title}.pdf"
    with open(pdf_path, "wb") as f:
        f.write(img2pdf.convert(image_paths))
    return pdf_path


def cleanup(path):
    path = path[:-12]
    if os.path.exists(path):
        shutil.rmtree(path)
    else:
        print(f"The path {path} does not exist.")
