from fastapi.responses import StreamingResponse
import io
import requests as req
from Utils.bot_evasion import get_cookies, load_cf_cookies, delete_cf_cookies


def proxy_image(url: str, header: str = None):

    cookies_dict = None
    if header:
        header = {"Referer": header}

    if "toongod" in url:
        cookies_dict = load_cf_cookies("https://www.toongod.org")
        header = {
            "User-Agent": "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36 OPR/119.0.0.0",
            "Accept": "image/avif,image/webp,image/apng,image/*,*/*;q=0.8",
            "Accept-Encoding": "gzip, deflate, br, zstd",
            "Accept-Language": "en-US,en;q=0.9",
            "Referer": "https://www.toongod.org/"
        }
    elif "toonily" in url:
        cookies_dict = load_cf_cookies("https://toonily.com")

    try:
        response = req.get(url, headers=header, stream=True,
                           cookies=cookies_dict if cookies_dict else None)
        if response.status_code == 200:
            return StreamingResponse(
                io.BytesIO(response.content),
                media_type=response.headers.get('content-type', 'image/jpeg')
            )
        return None
    except Exception as e:
        print(f"Error proxying image: {e}")
        return None
