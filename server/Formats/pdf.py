import img2pdf
from PIL import Image
import os
import shutil
from .image_downloader import download_chapter_images


def gen_pdf(images, chap_num, path, referer=None):
    """
    Generuje PDF dla rozdziału mangi na podstawie pobranych obrazów.

    Args:
        images (list): Lista URLi obrazów lub krotek (url, referer).
        chap_num (str): Numer rozdziału.
        path (str): Ścieżka do katalogu docelowego.
        referer (str, optional): Referer do pobierania obrazów.

    Returns:
        str: Ścieżka do wygenerowanego pliku PDF.
    """
    image_paths, ch_path = download_chapter_images(images, chap_num, path, referer)

    title = f"Chapter_{chap_num.replace('.', '_')}"
    pdf_path = f"{path}/{title}.pdf"

    try:
        with open(pdf_path, "wb") as f:
            f.write(img2pdf.convert(image_paths, rotation=img2pdf.Rotation.ifvalid))
        return pdf_path
    except Exception as e:
        shutil.rmtree(ch_path, ignore_errors=True)
        raise e
