import shutil
import os

def gen_cbz(images, chap_num, path, referer=None):
    """
    Downloads images and generates a CBZ for a manga chapter.

    Args:
        images (list): List of image URLs or tuples (url, referer) if referer is used.
        chap_num (str): Chapter number, used for naming the output CBZ and directory.
        path (str): Directory where the chapter folder and CBZ will be saved.
        referer (str, optional): If provided, sets the HTTP Referer header for image requests.

    Returns:
        str: Path to the generated CBZ file.

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
        return
    except Exception as e:
        shutil.rmtree(ch_path, ignore_errors=True)
        raise e