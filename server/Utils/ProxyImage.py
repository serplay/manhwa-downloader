from fastapi.responses import StreamingResponse
import io
import requests as req
from Utils.bot_evasion import get_cookies

def proxy_image(url: str, header: str = None):
    
    cookies_dict = None
    if header:
        header = {"Referer": header}
    try:
        response = req.get(url, headers=header, stream=True, cookies=cookies_dict if cookies_dict else None)
        if response.status_code == 200:
            return StreamingResponse(
                io.BytesIO(response.content),
                media_type=response.headers.get('content-type', 'image/jpeg')
            )
        return None
    except Exception as e:
        print(f"Error proxying image: {e}")
        return None
