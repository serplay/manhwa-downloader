import shutil
import os
from ebooklib import epub
from random import randint
import re




def gen_epub(path, update_progress, referer=None, comic_title="Comic"):
    """
    Generates a ePUB file for a manga based on the downloaded images.

    Args:
        path (str): Path to the target directory.
        update_progress (Optional[Callable]): Callback function to update progress, if available.

    Returns:
        str: Path to the generated ePUB file.
    """
    try:
        book = epub.EpubBook()
        book.set_identifier(f"id{randint(100000, 999999)}")
        book.set_title(comic_title)
        book.set_language("en")

        chapter_dirs = sorted(
            [d for d in os.scandir(path) if d.is_dir()],
            key=lambda e: float(re.findall(r"[\d.]+", e.name)[0])
        )
        total_chapters = len(chapter_dirs)
        spine = ["nav"]
        toc = []

        image_counter = 1  # to make image filenames unique
        
        if update_progress:
                update_progress(total_chapters, f"Creating ePUB...")

        for i, chapter in enumerate(chapter_dirs, 1):
            chapter_title = f"Chapter {chapter.name}"
            file_name = f"chap_{i:02}.xhtml"

            images = sorted(
                [f for f in os.scandir(chapter.path) if f.is_file()],
                key=lambda f: int(re.findall(r"\d+", f.name)[0])
            )

            html = f"<h1>{chapter_title}</h1>"

            for img in images:
                ext = os.path.splitext(img.name)[1].lower()
                mime = "image/jpeg" if ext in [".jpg", ".jpeg"] else "image/png"
                img_uid = f"image_{image_counter}"
                img_filename = f"images/{img_uid}{ext}"

                with open(img.path, "rb") as f:
                    img_content = f.read()

                epub_img = epub.EpubImage(
                    uid=img_uid,
                    file_name=img_filename,
                    media_type=mime,
                    content=img_content,
                )
                book.add_item(epub_img)
                html += f'<p><img src="{img_filename}" alt="Page {image_counter}"/></p>'

                image_counter += 1

            chap = epub.EpubHtml(title=chapter_title, file_name=file_name, lang="en")
            chap.content = html

            book.add_item(chap)
            spine.append(chap)
            toc.append(chap)

        # Add nav, TOC, and minimal CSS
        book.toc = toc
        book.add_item(epub.EpubNcx())
        book.add_item(epub.EpubNav())
        book.add_item(epub.EpubItem(
            uid="style_nav",
            file_name="style/nav.css",
            media_type="text/css",
            content="body { text-align: center; } img { max-width: 100%; height: auto; }"
        ))

        book.spine = spine
        epub.write_epub(f'{path}/{comic_title}.epub', book)
        return f'{path}/{comic_title}.epub'
    
    except Exception as e:
        shutil.rmtree(path, ignore_errors=True)
        raise e