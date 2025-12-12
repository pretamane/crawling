import requests
import time
import sys
import json

# Remote Server IP
BASE_URL = "http://13.229.251.25:8000"

def run_remote_demo():
    keyword = "history of the internet"
    print(f"🚀 Triggering REMOTE crawl on {BASE_URL} for: '{keyword}'...")
    
    # 1. Trigger Crawl
    try:
        resp = requests.post(f"{BASE_URL}/crawl", json={"keyword": keyword}, timeout=10)
        resp.raise_for_status()
        task_id = resp.json()["task_id"]
        print(f"✅ Crawl started! Task ID: {task_id}")
    except Exception as e:
        print(f"❌ Failed to start crawl: {e}")
        return

    # 2. Poll for results
    print("⏳ Polling for results...")
    for _ in range(15):  # Poll for 30 seconds max
        time.sleep(2)
        try:
            resp = requests.get(f"{BASE_URL}/crawl/{task_id}", timeout=10)
            resp.raise_for_status()
            data = resp.json()
            status = data["status"]
            
            if status == "completed":
                print("\n🎉 Remote Crawl Completed Successfully!")
                print(f"📂 Results found: {len(data['results'])}")
                
                if data['results']:
                    first_result = data['results'][0]
                    print("\n🥇 First Result:")
                    print(f"   Title: {first_result['title']}")
                    print(f"   Link:  {first_result['link']}")
                
                print("\n🧠 Trafilatura Extraction (Maxi Quality Check):")
                print(f"   Author: {data.get('meta_author')}")
                print(f"   Date:   {data.get('meta_date')}")
                
                text = data.get('extracted_text')
                if text:
                    print(f"   Text Length: {len(text)} chars")
                    print(f"   Snippet: {text[:200]}...")
                else:
                    print("   ❌ No text extracted!")
                    
                return
            else:
                print(f"   Status: {status}...", end="\r")
        except Exception as e:
            print(f"❌ Error polling: {e}")
            return

    print("\n❌ Timed out waiting for results.")

if __name__ == "__main__":
    run_remote_demo()
