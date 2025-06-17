from seleniumbase import SB

def get_with_captcha(url, elem, click=False):
    with SB(uc=True, xvfb=True) as sb:
        print("bim")
        sb.uc_open_with_reconnect(url, 4)
        print("bam")
        sb.uc_gui_click_captcha()
        print("bum")
        if elem:
            print("Waiting for element:", elem)
            try:
                sb.wait_for_element(elem, timeout=10)
            except Exception as e:
                raise Exception()
        if click:
            sb.click('button:contains("Show All Chapters")')
        soup = sb.get_beautiful_soup()
    return soup

def get_with_captcha_simple(url, elem, click=False):
    with SB(uc=True, xvfb=True) as sb:
        print("bim")
        sb.uc_open(url)
        print("bam")
        sb.uc_gui_click_captcha()
        print("bum")
        if elem:
            print("Waiting for element:", elem)
            try:
                sb.wait_for_element(elem, timeout=10)
            except Exception as e:
                raise Exception()
        if click:
            sb.click('button:contains("Show All Chapters")')
        soup = sb.get_beautiful_soup()
    return soup
