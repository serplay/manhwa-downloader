import img2pdf
import os
import shutil
import re
from PIL import Image
from zipfile import ZipFile

def gen_pdf(path, update_progress=None):
    """
    Generates a ZIP archive of PDFs for a manga based on the downloaded images.

    Args:
        path (str): Path to the target directory.
        update_progress (Optional[Callable]): Callback function to update progress, if available.

    Returns:
        str: Path to the generated Zip file.
    """
    chapter_paths = [f.path for f in os.scandir(path) if f.is_dir()]
    total_chapters = len(chapter_paths)
    pdfs = []
    if update_progress:
            update_progress(total_chapters, f"Creating PDFs...")
            
    for chapter in chapter_paths:
        chap_num = os.path.basename(chapter)
        pdf_path = f"{path}/{chap_num}.pdf"

        image_paths = sorted(
            [img.path for img in os.scandir(chapter) if img.is_file()],
            key=lambda p: int(re.search(r"(\d+)", os.path.basename(p)).group())
        )

        # Validate image sizes
        valid_images = []
        for img_path in image_paths:
            try:
                with Image.open(img_path) as im:
                    width, height = im.size
                    if width >= 72 and height >= 72:
                        valid_images.append(img_path)
            except Exception as e:
                print(f"Skipping invalid image {img_path}: {e}")

        with open(pdf_path, "wb") as f:
            f.write(img2pdf.convert(valid_images, rotation=img2pdf.Rotation.ifvalid))
        pdfs.append(pdf_path)
        shutil.rmtree(chapter,ignore_errors=True)
        
    if update_progress:
        update_progress(total_chapters, "Creating ZIP archive...")
    with ZipFile(f"{path}/Chapters.zip", 'w') as zipf:
        for pdf in pdfs:
            zipf.write(pdf, os.path.basename(pdf))
    return f"{path}/Chapters.zip"
    
    