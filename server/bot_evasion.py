from seleniumbase import SB

def get_with_captcha(url, elem, click=False):
    with SB(uc=True, xvfb=True) as sb:
        sb.uc_open_with_reconnect(url, 4)
        sb.uc_gui_click_captcha()
        if elem:
            sb.wait_for_element(elem, timeout=10)
        if click:
            sb.click('button:contains("Show All Chapters")')
        soup = sb.get_beautiful_soup()
    return soup