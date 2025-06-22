import os, shutil

def cleanup(path):
    """
    Removes a directory or file at the given path. If the path ends with '/Chapters.zip', the parent directory is removed instead.

    Args:
        path (str): Path to the directory or file to remove. If it ends with '/Chapters.zip', the parent directory is targeted for removal.

    Behavior:
        - If the path exists, it is removed recursively.
        - If the path does not exist, a message is printed.
    """
    path = path[:-12] if path.endswith("/Chapters.zip") else path
    if os.path.exists(path):
        shutil.rmtree(path)
    else:
        print(f"The path {path} does not exist.")