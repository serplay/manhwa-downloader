import img2pdf
import requests as req
import os

def get_chapter_images(ids, source):
    # Download images, save them temporarily
    try:
        source = int(source)
    except ValueError:
        return {"message": "Invalid source"}
    match source:
        case 0: #MangaDex
            athome_url = "https://api.mangadex.org/at-home/server/"
            for chap_id in ids:
                data = req.get(f"{athome_url}{chap_id}").json()
                baseUrl = data["baseUrl"]
                hash_url = data["chapter"]["hash"]
                images = data["chapter"]["data"]
                image_links = [f'{baseUrl}/data/{hash_url}/{image}' for image in images]
                print(image_links)
        case 1: #Manghuas
            return
        case 2: #Yakshascans
            return
        case 3: #Asurascan
            return
        case 4: #Kunmanga
            return
        case 5: #Toonily
            return
        case 6: #Toongod
            return
    
# generate PDF
def gen_pdf():
    pdf_path = f"{title}.pdf"
    with open(pdf_path, "wb") as f:
        f.write(img2pdf.convert(image_paths))
    return pdf_path

