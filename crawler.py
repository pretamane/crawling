import requests
from bs4 import BeautifulSoup
import time
import logging
import json
import trafilatura
from tenacity import retry, stop_after_attempt, wait_exponential, retry_if_exception_type
from database import SessionLocal, Task
from config import settings

# Configure logging
logging.basicConfig(level=getattr(logging, settings.LOG_LEVEL))
logger = logging.getLogger(__name__)

@retry(
    stop=stop_after_attempt(3),
    wait=wait_exponential(multiplier=1, min=4, max=10),
    retry=retry_if_exception_type(requests.RequestException)
)
def search_bing(keyword):
    """
    Searches Bing for the keyword and returns a list of results.
    Each result is a dictionary with 'title', 'link', and 'snippet'.
    """
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7",
        "Accept-Language": "en-US,en;q=0.9",
    }
    url = f"{settings.BING_SEARCH_URL}?format=rss&q={keyword}"
    
    try:
        response = requests.get(url, headers=headers, timeout=10)
        response.raise_for_status()
    except requests.RequestException as e:
        logger.error(f"Error searching Bing: {e}")
        raise # Re-raise for tenacity to catch

    soup = BeautifulSoup(response.content, 'xml')
    results = []
    
    # RSS items are in <item> tags
    for item in soup.find_all('item'):
        try:
            title = item.title.get_text() if item.title else "No title"
            link = item.link.get_text() if item.link else None
            snippet = item.description.get_text() if item.description else "No snippet"
            
            if link:
                results.append({
                    "title": title,
                    "link": link,
                    "snippet": snippet
                })
        except Exception as e:
            logger.warning(f"Error parsing result item: {e}")
            continue
            
    if not results:
        logger.warning("No results found. Saving XML to debug_bing.xml")
        with open("debug_bing.xml", "w", encoding="utf-8") as f:
            f.write(response.text)
            
    return results

    if not results:
        logger.warning("No results found. Saving XML to debug_bing.xml")
        with open("debug_bing.xml", "w", encoding="utf-8") as f:
            f.write(response.text)
            
    return results

@retry(
    stop=stop_after_attempt(3),
    wait=wait_exponential(multiplier=1, min=4, max=10),
    retry=retry_if_exception_type(Exception)
)
def fetch_page_content(url):
    """
    Fetches the URL using requests (with headers) and extracts content + metadata using Trafilatura.
    Returns a dict with 'html', 'text', 'meta_description', 'meta_author', 'meta_date'.
    """
    headers = {
        "User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7",
        "Accept-Language": "en-US,en;q=0.9",
    }
    
    try:
        # 1. Fetch with requests (more reliable control over headers)
        response = requests.get(url, headers=headers, timeout=15)
        response.raise_for_status()
        html_content = response.text
        
        result = {
            "html": html_content,
            "text": None,
            "meta_description": None,
            "meta_author": None,
            "meta_date": None
        }
        
        # 2. Extract with Trafilatura
        # extract_metadata expects the raw HTML string
        metadata = trafilatura.extract_metadata(html_content)
        if metadata:
            result["meta_description"] = metadata.description
            result["meta_author"] = metadata.author
            result["meta_date"] = metadata.date
            
        result["text"] = trafilatura.extract(
            html_content, 
            include_comments=False, 
            include_tables=True, 
            with_metadata=False
        )
        
        return result
    except Exception as e:
        logger.error(f"Error fetching page {url}: {e}")
        raise # Re-raise for tenacity

def process_crawl_task(task_id, keyword):
    """
    Background task to perform the crawl and update the database.
    """
    logger.info(f"Starting task {task_id} for keyword: {keyword}")
    
    db = SessionLocal()
    try:
        # 1. Search Bing
        try:
            search_results = search_bing(keyword)
        except Exception as e:
            logger.error(f"Search failed after retries: {e}")
            search_results = []
        
        first_page_data = None
        if search_results:
            # 2. Fetch first result
            first_url = search_results[0]['link']
            logger.info(f"Fetching first result: {first_url}")
            try:
                first_page_data = fetch_page_content(first_url)
            except Exception as e:
                logger.error(f"Page fetch failed after retries: {e}")
                first_page_data = {"html": f"Error fetching page: {e}"}
        else:
            logger.warning("No search results found.")
        
        # Update task in DB
        task = db.query(Task).filter(Task.id == task_id).first()
        if task:
            task.status = "completed"
            task.results_json = json.dumps(search_results)
            
            if first_page_data:
                task.first_page_html = first_page_data.get("html")
                task.extracted_text = first_page_data.get("text")
                task.meta_description = first_page_data.get("meta_description")
                task.meta_author = first_page_data.get("meta_author")
                task.meta_date = first_page_data.get("meta_date")
                
            db.commit()
            logger.info(f"Task {task_id} completed and saved to DB.")
        else:
            logger.error(f"Task {task_id} not found in DB.")
            
    except Exception as e:
        logger.error(f"Error processing task {task_id}: {e}")
    finally:
        db.close()
