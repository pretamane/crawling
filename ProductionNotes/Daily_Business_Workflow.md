# 💼 Daily Business Workflow: Single Laptop Mode
## (Fortune Wifi Residential IP)

This guide documents the daily routine for running your crawling business using your laptop as the primary worker node and AWS as the central brain.

---

### 🌅 Phase 1: Morning Startup (Connect & Launch)

**Step 1: Open the "Tunnel" Terminal**
This connects your laptop to the AWS Brain. Keep this terminal open all day.
```bash
# SSH Tunnel: Maps AWS Redis/DB to your Localhost
ssh -i sg-crawling-key.pem -N -o StrictHostKeyChecking=no \
    -L 6380:localhost:6379 \
    -L 5433:localhost:5432 \
    -L 9005:localhost:9000 \
    ubuntu@54.179.175.198
```

**Step 2: Start the "Worker" Terminal**
This starts the actual crawling engine on your laptop.
```bash
# Configure Environment to use the Tunnel
export REDIS_URL=redis://127.0.0.1:6380
export DATABASE_URL=postgres://crawler:crawler_password@127.0.0.1:5433/crawling_db
export MINIO_ENDPOINT=http://127.0.0.1:9005
export MINIO_ROOT_USER=minio_user
export MINIO_ROOT_PASSWORD=minio_password
export MINIO_BUCKET=crawler-data
export STORAGE_PATH=./results
export RUST_LOG=info
export PORT=3001

# Run the Crawler
./rust-crawler
```
*You should see: `👷 Worker started, polling Redis...`*

---

### 🏭 Phase 2: Production (Feeding the Beast)

You don't want to use `curl` for 1000 keywords. Use this Python script to bulk-load jobs.

**Create `bulk_trigger.py`:**
```python
import requests
import json

# Configuration
API_URL = "http://54.179.175.198:3000/crawl"  # AWS Public API
KEYWORDS = [
    "Yangon Real Estate Prices",
    "Best Hotels in Mandalay",
    "Myanmar Currency Exchange Rates",
    "Used Cars for Sale Yangon"
]

print(f"🚀 Injecting {len(KEYWORDS)} jobs into the Global Queue...")

for kw in KEYWORDS:
    payload = {
        "keyword": kw,
        "engine": "google"  # or "bing", "generic"
    }
    try:
        r = requests.post(API_URL, json=payload, timeout=5)
        if r.status_code == 200:
            print(f"✅ Queued: {kw}")
        else:
            print(f"❌ Failed: {kw} ({r.status_code})")
    except Exception as e:
        print(f"⚠️ Error: {e}")
```
**Run it:** `python3 bulk_trigger.py`

---

### 📊 Phase 3: Monitoring & Export

**Check Progress:**
Go to your Adminer Dashboard: [http://localhost:8081](http://localhost:8081)
*(Requires the Adminer SSH Tunnel on port 8081)*

**Export Data to CSV (Business Report):**
Run this query in Adminer (SQL Command):
```sql
SELECT keyword, status, created_at, 
       results_json->'results'->0->>'title' as top_result 
FROM tasks 
WHERE status = 'completed' 
ORDER BY created_at DESC;
```
*Then click "Export" at the bottom.*

---

### 💡 Pro Tips
*   **Speed**: Your single laptop can handle ~5-10 pages per minute depending on delay settings.
*   **Scale**: Want to go faster? Open another terminal, source the same exports, and run `./rust-crawler` again. (2 Workers on 1 Laptop!).
*   **Safety**: Since you are using "Fortune Wifi" (Residential), Google trusts you more than AWS. You can be deeper and faster.
