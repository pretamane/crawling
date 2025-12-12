import requests
import time
import sys
import json

BASE_URL = "http://127.0.0.1:8000"

def run_demo():
    keyword = "top tourist attractions in Paris 2025"
    print(f"🚀 Triggering crawl for: '{keyword}'...")
    
    # 1. Trigger Crawl
    try:
        resp = requests.post(f"{BASE_URL}/crawl", json={"keyword": keyword})
        resp.raise_for_status()
        task_id = resp.json()["task_id"]
        print(f"✅ Crawl started! Task ID: {task_id}")
    except Exception as e:
        print(f"❌ Failed to start crawl: {e}")
        return

    # 2. Poll for results
    print("⏳ Polling for results...")
    for _ in range(10):  # Poll for 20 seconds max
        time.sleep(2)
        try:
            resp = requests.get(f"{BASE_URL}/crawl/{task_id}")
            resp.raise_for_status()
            data = resp.json()
            status = data["status"]
            
            if status == "completed":
                print("\n🎉 Crawl Completed Successfully!")
                print(f"📂 Results found: {len(data['results'])}")
                
                if data['results']:
                    first_result = data['results'][0]
                    print("\n🥇 First Result:")
                    print(f"   Title: {first_result['title']}")
                    print(f"   Link:  {first_result['link']}")
                    print(f"   Snippet: {first_result['snippet'][:100]}...")
                
                if data.get('first_page_html'):
                    print(f"\n📄 HTML Content Captured: Yes ({len(data['first_page_html'])} bytes)")
                else:
                    print("\n⚠️ No HTML content captured.")
                    
                print("\n🧠 Trafilatura Extraction:")
                print(f"   Author: {data.get('meta_author')}")
                print(f"   Date:   {data.get('meta_date')}")
                print(f"   Desc:   {data.get('meta_description')}")
                if data.get('extracted_text'):
                    print(f"   Text:   {data.get('extracted_text')[:200]}...")
                else:
                    print("   Text:   None")
                return
            else:
                print(f"   Status: {status}...", end="\r")
        except Exception as e:
            print(f"❌ Error polling: {e}")
            return

    print("\n❌ Timed out waiting for results.")

if __name__ == "__main__":
    run_demo()
