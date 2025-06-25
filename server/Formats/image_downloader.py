import requests as req
import os
import shutil
from PIL import Image

def download_chapter_images(images, chap_num, path, referer=None):
    """
    Downloads a list of chapter images to a local directory, handling possible corruption and format conversion.

    Args:
        images (list): A list of image URLs. If `referer` is True, each item must be a tuple (url, referer_url),
                       otherwise just the image URL string.
        chap_num (str or int): The chapter number or identifier, used to name the subdirectory.
        path (str): The base directory where chapter images will be saved.
        referer (bool, optional): Whether to use referer headers for requests. If True, expects tuples in `images`.

    Returns:
        tuple:
            - image_paths (list of str): Paths to successfully downloaded and verified image files.
            - ch_path (str): Path to the directory containing downloaded images.

    Raises:
        Exception: If a critical error occurs during download, conversion, or file operations.
    
    Notes:
        - Converts `.webp` images to `.jpg` format and removes the original `.webp` files.
        - Skips corrupted images or images smaller than 72x72 pixels.
        - In case of a total failure, the created chapter directory is deleted.
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
            except req.RequestException:
                is_corrupted = True

            if not is_corrupted:
                extension = img_url.split(".")[-1].split("?")[0]
                img_path = f"{ch_path}/{i}.{extension}"
                with open(img_path, "wb") as img_file:
                    img_file.write(img_data)
            else:
                extension = "png"
                img_path = os.path.join(os.path.dirname(__file__), "corrupt.png")

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
        return image_paths, ch_path
    except Exception as e:
        shutil.rmtree(ch_path, ignore_errors=True)
        raise e 