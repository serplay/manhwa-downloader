import os
import shutil
import re
import subprocess

def gen_cbr(path, update_progress=None, comic_title="Comic"):
    """
    Generates CBR files for each chapter and packs them into a single RAR archive (Chapters.rar).

    Args:
        path (str): Path to the target directory.
        update_progress (Optional[Callable]): Progress callback.

    Returns:
        str: Path to the generated Chapters.rar file.
    """
    try:
        chapter_dirs = [entry.path for entry in os.scandir(path) if entry.is_dir()]
        total_chapters = len(chapter_dirs)
        cbr_files = []

        if update_progress:
            update_progress(total_chapters, "Creating CBR...")

        for i, chapter_path in enumerate(chapter_dirs):
            chapter_name = os.path.basename(chapter_path)
            cbr_name = f"Chapter {chapter_name}.cbr"
            cbr_path = os.path.join(path, cbr_name)

            if update_progress:
                update_progress(i + 1, f"Processing chapter {chapter_name}...")

            image_files = sorted(
                [f for f in os.scandir(chapter_path) if f.is_file()],
                key=lambda f: int(re.search(r"(\d+)", f.name).group())
            )
            image_names = [f.name for f in image_files]

            try:
                subprocess.run(
                    ["rar", "a", "-ep1", "temp.rar"] + image_names,
                    cwd=chapter_path,
                    check=True
                )

                shutil.move(os.path.join(chapter_path, "temp.rar"), cbr_path)
                cbr_files.append(cbr_path)
                shutil.rmtree(chapter_path)

            except subprocess.CalledProcessError as e:
                print(f"Failed to create CBR for {chapter_name}: {e}")

        subprocess.run(
            ["rar", "a", "-ep1", "Chapters.rar"] + [os.path.basename(cbr) for cbr in cbr_files],
            cwd=path,
            check=True
        )

        for cbr in cbr_files:
            os.remove(cbr)

        return f"{path}/Chapters.rar"

    except Exception as e:
        raise RuntimeError(f"Error generating CBRs: {e}")
