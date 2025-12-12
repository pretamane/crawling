import requests
import sys

task_id = "f609179c-0a15-403c-9471-a67a285f08e9"
url = f"http://127.0.0.1:8000/crawl/{task_id}"

try:
    response = requests.get(url)
    response.raise_for_status()
    data = response.json()
    
    print(f"Status: {data['status']}")
    print(f"Keyword: {data['keyword']}")
    print(f"Number of results: {len(data['results'])}")
    
    if data['results']:
        print(f"First result title: {data['results'][0]['title']}")
        print(f"First result link: {data['results'][0]['link']}")
        
    if data.get('first_page_html'):
        print("First page HTML: Present")
        print(f"HTML length: {len(data['first_page_html'])}")
    else:
        print("First page HTML: Missing")
        
except Exception as e:
    print(f"Error: {e}")
