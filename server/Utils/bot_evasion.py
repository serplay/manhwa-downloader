from seleniumbase import SB
import redis
import os
import json


redis_url =  os.getenv("REDIS_URL", "redis://redis:6379/1")

redis_client = redis.Redis.from_url(redis_url)


def get_with_captcha(url: str, elem: str, click: bool = False) -> dict:
    """
    Opens a webpage with CAPTCHA protection and returns its HTML content as a BeautifulSoup object.

    Args:
        url (str): The URL of the webpage to open.
        elem (str): A CSS selector string for an element to wait for after solving the CAPTCHA.
        click (bool, optional): If True, clicks the "Show All Chapters" button after loading. Defaults to False.

    Returns:
        dict: A BeautifulSoup object of the page content if successful, otherwise an empty dictionary.
    """
    with SB(uc=True, xvfb=True) as sb:
        sb.uc_open_with_reconnect(url, 4)
        sb.uc_gui_click_captcha()
        if elem:
            print("Waiting for element:", elem)
            try:
                sb.wait_for_element(elem, timeout=10)
            except Exception as e:
                return {}
        if click:
            sb.click('button:contains("Show All Chapters")')
        soup = sb.get_beautiful_soup()
    return soup


def get_cookies(url: str) -> dict:
    """
    Retrieves Cloudflare-related cookies from a given URL using a headless SeleniumBase browser.

    Args:
        url (str): The URL to retrieve cookies from.

    Returns:
        dict: A dictionary of cookie name-value pairs where the name contains 'cf'.

    Raises:
        Exception: If the cookies could not be retrieved.
    """
    try:
        with SB(uc=True, xvfb=True) as sb:
            sb.uc_open_with_reconnect(url)
            sb.uc_gui_click_captcha()
            sb.wait_for_element("body", timeout=15)
            selenium_cookies = sb.driver.get_cookies()
            cf_cookies = {cookie["name"]: cookie["value"]
                          for cookie in selenium_cookies if "cf" in cookie["name"]}
            save_cf_cookies(url, cf_cookies)
            return cf_cookies
    except Exception as e:
        raise Exception(
            f"Failed to retrieve cookies from Toongod using SeleniumBase: {e}")


def save_cf_cookies(domain: str, cookies: dict) -> None:
    """
    Save Cloudflare cookies to Redis.

    Args:
        domain (str): Domain for which cookies are saved.
        cookies (str): JSON string of cookies.
    """
    try:
        redis_client.set(f"cf_cookies:{domain}", json.dumps(cookies), ex=1800)
        print(f"DEBUG: Cookies for {domain} saved successfully." if os.getenv(
            "DEBUG") else "")
    except Exception as e:
        print(f"Error saving cookies for {domain}: {e}")


def load_cf_cookies(domain: str) -> dict:
    """
    Load Cloudflare cookies from Redis.

    Args:
        domain (str): Domain for which cookies are loaded.

    Returns:
        dict: Dictionary of cookies if found, otherwise an empty dictionary.
    """
    try:
        cookies = redis_client.get(f"cf_cookies:{domain}")
        if cookies:
            return json.loads(cookies)
        return {}
    except Exception as e:
        print(f"Error loading cookies for {domain}: {e}")
        return {}


def delete_cf_cookies(domain: str) -> None:
    """
    Delete Cloudflare cookies from Redis.

    Args:
        domain (str): Domain for which cookies are deleted.
    """
    try:
        redis_client.delete(f"cf_cookies:{domain}")
        print(f"DEBUG: Cookies for {domain} deleted successfully." if os.getenv(
            "DEBUG") else "")
    except Exception as e:
        print(f"Error deleting cookies for {domain}: {e}")
