from pathlib import Path
from zipfile import ZipFile
import os
import shutil
from math import ceil
import re
from cbz.comic import ComicInfo
from cbz.constants import PageType, YesNo, Manga, AgeRating, Format
from cbz.page import PageInfo

def gen_cbz(path, update_progress=None, comic_title="Comic"):
    """
    Generates a CBZ files for a manga based on the downloaded images.

    Args:
        path (str): Path to the target directory.
        update_progress (Optional[Callable]): Callback function to update progress, if available.

    Returns:
        str: Path to the generated CBZ file.
    """
    try:
        chapter_paths = [f.path for f in os.scandir(path) if f.is_dir()]
        total_chapters = len(chapter_paths)

        if update_progress:
                update_progress(total_chapters, f"Creating CBZ...")

        chapters = []
        for i,chapter in enumerate(chapter_paths):
            chap_num = os.path.basename(chapter)
            if update_progress:
                update_progress(total_chapters, f"Processing chapter {chap_num}...")
            images = sorted([img.path for img in os.scandir(chapter) if img.is_file()],key=lambda p: int(re.search(r"(\d+)", os.path.basename(p)).group()))
            pages = [
                PageInfo.load(
                    path=img_path,
                    type=PageType.FRONT_COVER if i == 0 else PageType.BACK_COVER if i == len(images) - 1 else PageType.STORY,
                )
                for i, img_path in enumerate(images)
            ]

            comic = ComicInfo(
                pages=pages,
                title="Chapter " + chap_num,
                serites=comic_title,
                manga=Manga.NO,
                number=i+1,
                language_iso="en",
                format=Format.WEB_COMIC,
                black_and_white=YesNo.NO,
                age_rating=AgeRating.UNKNOWN,
            )

            cbz_content = comic.pack()
            cbz_path = Path(path) / f"Chapter {chap_num}.cbz"
            chapters.append(cbz_path)
            cbz_path.write_bytes(cbz_content)
            shutil.rmtree(chapter,ignore_errors=True)


        with ZipFile(f"{path}/Chapters.zip", 'w') as zipf:
            for cbz in chapters:
                zipf.write(cbz, os.path.basename(cbz))
                os.remove(cbz)
    except Exception as e:
        raise e


    return f"{path}/Chapters.zip"
    
    