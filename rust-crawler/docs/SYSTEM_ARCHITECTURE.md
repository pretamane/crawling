# System Architecture & Workflow Analysis

## 1. High-Level Architecture Overview
This system is a distributed, stealth-oriented web crawler built in Rust. It mimics human behavior to scrape data from anti-bot platforms (Facebook, Google) and integrates with an **AI/ML Sidecar** for intelligent content analysis.

### Service Interaction Map
```mermaid
graph TD
    User([User/Client]) -->|HTTP JSON| API[Axum API Node]
    
    subgraph "Infrastructure Layer"
        Redis[(Redis Queue)] 
        DB[(Supabase PostgreSQL)]
    end
    
    subgraph "Execution Layer"
        Worker[Rust Worker Service]
        Chrome[Headless Chrome Runtime]
    end

    subgraph "Intelligence Layer"
        Python[Python ML Sidecar]
        Model1[Spacy NER Model]
        Model2[FastText Classifier]
        Python --> Model1
        Python --> Model2
    end
    
    API -->|TCP/RESP| Redis
    API -->|TCP/SQL| DB
    
    Worker -->|TCP/RESP| Redis
    Worker -->|WebSocket/CDP| Chrome
    Worker -->|HTTP/REST| Python
    Worker -->|TCP/SQL| DB
    
    Chrome -->|HTTPS/TLS| Internet(Target Websites)
    Worker -.->|Read| Cookies[cookies.json]
```

---

## 2. Tool & Service Interconnections

1.  **API/Worker <-> Redis (`TCP / RESP`)**
    *   **Mechanism**: Rust `redis-rs` pushes jobs to `queue:jobs`. Decouples ingestion from processing.

2.  **Worker <-> Chrome (`WebSocket / CDP`)**
    *   **Mechanism**: Rust `headless_chrome` spawns `chrome` and controls it via DevTools Protocol. Drives navigation, injection, and extraction.

3.  **Worker <-> Python Sidecar (`HTTP / REST`)**
    *   **Why**: Rust is great for speed/safety, but Python is king for ML libraries.
    *   **Mechanism**: Rust Worker makes HTTP POST requests to `localhost:8000` (FastAPI).
    *   **Payload**: `{ "text": "extracted content..." }`.
    *   **Response**: `{ "entities": [...], "category": "Tech", "confidence": 0.98 }`.

4.  **Worker <-> Supabase (`TCP / Postgres Wire`)**
    *   **Mechanism**: Rust `sqlx` connects to the Transaction Pooler (port 6543). Stores final enriched data.

---

## 3. Workflow Lifecycle (Step-by-Step)

### Phase 1: Ingestion
1.  **User**: `POST /crawl` -> API.
2.  **API**: Generates `task_id`, saves `pending` status to DB, pushes to Redis.

### Phase 2: Execution & Stealth
3.  **Worker**: Pops job, loads `cookies.json` for the target domain.
4.  **Browser**: Launches in Headless Mode with stealth flags.
5.  **Safety**: Checks for Captchas. Sleeps randomly (5-12s) to mimic human latency.
6.  **Navigation**: Human-like scrolling to trigger lazy-loads (especially for Facebook).

### Phase 3: Extraction
7.  **JS Injection**: Runs scripts in-browser to identify "Selling Points" (Headlines, Benefits) and Feed Posts.
8.  **Data Return**: structured JSON returns to Rust.

### Phase 4: AI Enrichment (The Brain)
9.  **Sentiment**: Rust runs a native keyword-based sentiment analysis (`Positive`/`Negative`).
10. **NER**: Rust sends text to Python Sidecar to extract People, Orgs, Locations using **Spacy**.
11. **Classification**: Rust sends text to Python Sidecar to categorize content (e.g., "Business", "Tech").

### Phase 5: Persistence
12. **Storage**: Enriched data (Marketing + ML Tags) is saved to Supabase `tasks` table (`marketing_data` column).

---

## 4. Sequence Diagram (The Flow)

```mermaid
sequenceDiagram
    participant U as User
    participant A as API
    participant R as Redis
    participant W as Worker
    participant C as Chrome
    participant P as Python ML
    participant D as Supabase DB
    
    U->>A: POST /crawl
    A->>R: Push Job
    A->>U: 200 OK
    
    W->>R: Pop Job
    W->>C: Launch & Navigate (Stealth)
    C-->>W: Raw Text & HTML
    
    rect rgb(240, 248, 255)
        Note over W, P: AI Enrichment Loop
        W->>W: Analyze Sentiment (Native)
        W->>P: POST /ml/ner
        P-->>W: Entities [Elon Musk, SpaceX]
        W->>P: POST /ml/classify
        P-->>W: Category "Technology"
    end
    
    W->>D: UPDATE 'completed' with Enriched Data
```

---

## 5. Security & Data
*   **Credentials**: `cookies.json` and `.env` are git-ignored.
*   **Isolation**: Browser runs in a sandbox; ML runs in a local sidecar. No data leaves your infrastructure.
