import requests
import time
import json
import csv
from datetime import datetime
import os

# Configuration
BASE_URL = "http://13.229.251.25:8000"  # Your AWS Instance IP
OUTPUT_DIR = "/home/guest/tzdump"

def save_data(data, keyword):
    """Saves the crawled data to JSON, CSV, and Text files."""
    if not os.path.exists(OUTPUT_DIR):
        os.makedirs(OUTPUT_DIR)
    
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    safe_keyword = keyword.replace(" ", "_")
    base_filename = f"{OUTPUT_DIR}/{safe_keyword}_{timestamp}"

    # 1. Save Full JSON (Raw Data)
    json_filename = f"{base_filename}.json"
    with open(json_filename, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
    print(f"✅ Saved full JSON to: {json_filename}")

    # 2. Save Summary to CSV (Excel compatible)
    csv_filename = f"{base_filename}.csv"
    with open(csv_filename, "w", newline="", encoding="utf-8") as f:
        writer = csv.writer(f)
        writer.writerow(["Title", "Link", "Snippet"])
        for result in data.get("results", []):
            writer.writerow([result.get("title"), result.get("link"), result.get("snippet")])
    print(f"✅ Saved summary CSV to: {csv_filename}")

    # 3. Save Extracted Text (Readable Content)
    if data.get("extracted_text"):
        text_filename = f"{base_filename}.txt"
        with open(text_filename, "w", encoding="utf-8") as f:
            f.write(f"Title: {keyword}\n")
            f.write(f"Date: {data.get('meta_date')}\n")
            f.write(f"Author: {data.get('meta_author')}\n")
            f.write(f"URL: {data.get('results', [{}])[0].get('link')}\n")
            f.write("-" * 40 + "\n\n")
            f.write(data.get("extracted_text"))
        print(f"✅ Saved extracted text to: {text_filename}")

def pretty_print(data):
    """Pretty prints the key information to the console."""
    print("\n" + "="*60)
    print(f"🔍 CRAWL RESULT: {data.get('keyword')}")
    print("="*60)
    
    print(f"\n📊 Metadata:")
    print(f"   • Status: {data.get('status')}")
    print(f"   • Results Found: {len(data.get('results', []))}")
    print(f"   • Author: {data.get('meta_author')}")
    print(f"   • Date: {data.get('meta_date')}")
    
    print(f"\n📝 Extracted Content Preview:")
    text = data.get('extracted_text', '')
    if text:
        preview = text[:500].replace('\n', ' ')
        print(f"   \"{preview}...\"")
    else:
        print("   (No text extracted)")

    print(f"\n🔗 Top 3 Links:")
    for i, res in enumerate(data.get('results', [])[:3]):
        print(f"   {i+1}. {res['title']}")
        print(f"      {res['link']}")

def main():
    if len(sys.argv) > 1:
        keyword = " ".join(sys.argv[1:])
    else:
        keyword = input("Enter keyword to crawl: ")

    print(f"🚀 Starting crawl for: '{keyword}'...")
    
    try:
        # Trigger
        resp = requests.post(f"{BASE_URL}/crawl", json={"keyword": keyword})
        resp.raise_for_status()
        task_id = resp.json()["task_id"]
        print(f"⏳ Task ID: {task_id} (Polling...)")
        
        # Poll
        for _ in range(20):
            time.sleep(2)
            resp = requests.get(f"{BASE_URL}/crawl/{task_id}")
            data = resp.json()
            if data["status"] == "completed":
                pretty_print(data)
                save_data(data, keyword)
                return
            print(".", end="", flush=True)
            
        print("\n❌ Timed out.")
        
    except Exception as e:
        print(f"\n❌ Error: {e}")

if __name__ == "__main__":
    import sys
    main()
