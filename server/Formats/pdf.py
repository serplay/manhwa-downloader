import requests as req
import os
import shutil
import img2pdf
from PIL import Image


def gen_pdf(images, chap_num, path, referer=None):
    """
    Downloads images and generates a PDF for a manga chapter.

    Args:
        images (list): List of image URLs or tuples (url, referer) if referer is used.
        chap_num (str): Chapter number, used for naming the output PDF and directory.
        path (str): Directory where the chapter folder and PDF will be saved.
        referer (str, optional): If provided, sets the HTTP Referer header for image requests.

    Returns:
        str: Path to the generated PDF file.

    Behavior:
        - Downloads each image, handling referer headers if needed.
        - Converts WEBP images to JPEG.
        - Skips images smaller than 72x72 pixels.
        - If an image fails to download, uses a placeholder 'corrupt.png'.
        - Cleans up temporary images if an error occurs.
    """
    image_paths = []
    ch_path = f"{path}/{chap_num}"
    os.makedirs(ch_path, exist_ok=True)

    try:
        for i, img_url in enumerate(images):
            is_corrupted = False
            try:
                if referer:
                    headers = {'Referer': img_url[1]}
                    img_url = img_url[0]
                img_resp = req.get(img_url, timeout=10, headers=headers if referer else None)
                img_resp.raise_for_status()
                img_data = img_resp.content
            except req.RequestException as e:
                is_corrupted = True
                
            if not is_corrupted:
                extension = img_url.split(".")[-1].split("?")[0]  # handle URLs with query params
                img_path = f"{ch_path}/{i}.{extension}"

                with open(img_path, "wb") as img_file:
                    img_file.write(img_data)
            else:
                extension = "png"
                img_path = "Formats/corrupt.png"
            
            if extension.lower() == "webp":
                try:
                    im = Image.open(img_path).convert("RGB")
                    jpg_path = f'{img_path}.jpg'
                    im.save(jpg_path, "JPEG")
                    os.remove(img_path)
                    with Image.open(jpg_path) as img:
                        if img.size[0] < 72 or img.size[1] < 72:
                            continue
                    image_paths.append(jpg_path)
                except Exception as e:
                    raise Exception(f"Failed to convert WEBP to JPEG: {e}")
            else:
                with Image.open(img_path) as img:
                    if img.size[0] < 72 or img.size[1] < 72:
                        continue
                image_paths.append(img_path)

        title = f"Chapter_{chap_num.replace('.', '_')}"
        pdf_path = f"{path}/{title}.pdf"

        with open(pdf_path, "wb") as f:
            f.write(img2pdf.convert(image_paths, rotation=img2pdf.Rotation.ifvalid))

        return pdf_path

    except Exception as e:
        # Clean up images if something fails
        shutil.rmtree(ch_path, ignore_errors=True)
        raise e