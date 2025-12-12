# Bing Crawling API - Developer Handoff

## 1. Project Overview
This is a production-grade FastAPI service that:
1.  **Searches Bing** for travel/tourism keywords (using RSS to bypass bots).
2.  **Crawls** the results asynchronously.
3.  **Extracts "Maxi Quality" Content**: Uses `trafilatura` to get clean text, author, date, and description.
4.  **Persists Data**: Stores everything in a SQLite database (`crawling.db`).

## 2. Server Access (AWS)
- **Instance Name**: `sg crawling`
- **Region**: `ap-southeast-1` (Singapore)
- **Public IP**: `13.229.251.25`
- **SSH User**: `ubuntu`
- **Key File**: `sg-crawling-key.pem` (Located in project root)

### SSH Connection
```bash
ssh -i sg-crawling-key.pem ubuntu@13.229.251.25
```

## 3. Deployment Architecture
- **Container**: Dockerized FastAPI app running on port `8000`.
- **Reverse Proxy**: **Caddy** handles HTTPS and forwards requests to port `8000`.
- **SSL**: Automatic SSL via Let's Encrypt (managed by Caddy).
- **Database**: SQLite file inside the container (mounted volume recommended for persistence in future).

### Service Status
- **Swagger UI**: [https://13.229.251.25.nip.io/docs](https://13.229.251.25.nip.io/docs)
- **ReDoc**: [https://13.229.251.25.nip.io/redoc](https://13.229.251.25.nip.io/redoc)

## 4. API Reference

### `POST /crawl`
Triggers a background crawl task.

**Request**:
```json
{
  "keyword": "best street food in singapore"
}
```

**Response**:
```json
{
  "task_id": "550e8400-e29b-41d4-a716-446655440000",
  "message": "Crawl started"
}
```

### `GET /crawl/{task_id}`
Retrieves the status and results of a task.

**Response (Completed)**:
```json
{
  "status": "completed",
  "keyword": "best street food in singapore",
  "created_at": "2025-12-11T10:00:00",
  "results": [
    {
      "title": "Top 10 Street Food...",
      "link": "https://example.com/food",
      "snippet": "Laksa is a spicy noodle soup..."
    }
  ],
  "first_page_html": "<html>...</html>",
  "extracted_text": "Full clean article text...",
  "meta_description": "A guide to Singapore's best eats.",
  "meta_author": "Jane Doe",
  "meta_date": "2025-01-15"
}
```

## 5. Data Handling Script
Use `save_results.py` to crawl and save data locally.

```bash
# Usage
python3 save_results.py "your keyword"

# Output Locations
# - /home/guest/tzdump/keyword_date.json
# - /home/guest/tzdump/keyword_date.csv
```

## 6. Maintenance & Troubleshooting

### Restarting the App
```bash
# SSH into server
ssh -i sg-crawling-key.pem ubuntu@13.229.251.25

# Restart Docker Container
sudo docker restart bing-crawler-container
```

### Viewing Logs
```bash
sudo docker logs -f bing-crawler-container
```

### Updating Code
1.  Make changes locally.
2.  Tar and SCP to server:
    ```bash
    tar -czf app.tar.gz main.py crawler.py ...
    scp -i sg-crawling-key.pem app.tar.gz ubuntu@13.229.251.25:~/bing-crawler/
    ```
3.  Rebuild and Run:
    ```bash
    cd bing-crawler
    tar -xzf app.tar.gz
    sudo docker build -t bing-crawler .
    sudo docker stop bing-crawler-container
    sudo docker rm bing-crawler-container
    sudo docker run -d -p 8000:8000 --name bing-crawler-container bing-crawler
    ```
