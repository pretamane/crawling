# 🦅 Hybrid CI/CD: GitHub Repo + Self-Hosted CI
## (Keeping GitHub, Replacing Actions)

Yes! You can absolutely keep your code on **GitHub** but run your build/deploy pipeline on your own storage/server using **Woodpecker CI**.

### Why Woodpecker?
*   It connects directly to GitHub (you log in with your GitHub button).
*   It ignores `.github/workflows`.
*   It uses its own `.woodpecker.yml` (simpler, container-based syntax).
*   It runs entirely on your own AWS/Laptop.

---

### 1️⃣ Setup Woodpecker on AWS
Run this `docker-compose.yml` on your server to start the CI engine.

```yaml
version: '3'

services:
  woodpecker-server:
    image: woodpeckerci/woodpecker-server:latest
    ports:
      - 8000:8000
    environment:
      - WOODPECKER_HOST=http://54.179.175.198:8000
      - WOODPECKER_GITHUB=true
      - WOODPECKER_GITHUB_CLIENT=YOUR_GITHUB_CLIENT_ID
      - WOODPECKER_GITHUB_SECRET=YOUR_GITHUB_CLIENT_SECRET
      - WOODPECKER_AGENT_SECRET=super_secret_key
    volumes:
      - woodpecker-server-data:/var/lib/woodpecker

  woodpecker-agent:
    image: woodpeckerci/woodpecker-agent:latest
    command: agent
    restart: always
    environment:
      - WOODPECKER_SERVER=woodpecker-server:9000
      - WOODPECKER_AGENT_SECRET=super_secret_key
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock

volumes:
  woodpecker-server-data:
```

### 2️⃣ Connect GitHub to Woodpecker (The "Handshake" 🤝)
Woodpecker needs permission to see your private code. We give it a Key (Client ID) and a Password (Client Secret).

**Step-by-Step on GitHub.com:**
1.  Log in to GitHub and click your profile photo -> **Settings**.
2.  Scroll down to the left sidebar bottom: **Developer settings**.
3.  Click **OAuth Apps** -> **New OAuth App**.
4.  **Fill in the Form EXACTLY like this**:
    *   **Application Name**: `AWS Woodpecker CI`
    *   **Homepage URL**: `http://54.179.175.198:8000`
    *   **Authorization callback URL**: `http://54.179.175.198:8000/authorize`
5.  Click **Register application**.
6.  **CRITICAL STEP**: On the next screen, click **Generate a new client secret**.
7.  **Keep this tab open!** You will see a `Client ID` (starts with `Iv1...`) and a `Client Secret` (long string).

### 3️⃣ Add Keys to Your Server
Now we tell your server these secret codes.

1.  **SSH In**: `ssh -i sg-crawling-key.pem ubuntu@54.179.175.198`
2.  **Edit Config**: `nano docker-compose.woodpecker.yml`
3.  **Navigate**: Use arrow keys to find `WOODPECKER_GITHUB_CLIENT` and `SECRET`.
4.  **Paste**: Delete the temporary text and paste your actual keys from GitHub.
    *   *Tip*: To paste in terminal, use `Right Click` or `Shift+Insert`.
5.  **Save & Exit**:
    *   Press `Ctrl + X`
    *   Press `Y` (for Yes)
    *   Press `Enter`

### 4️⃣ Launch!
Reload the specific Woodpecker project to apply the keys (leaving the crawler alone).

```bash
docker compose -f ~/docker-compose.woodpecker.yml -p woodpecker up -d
```

### 5️⃣ Final Verification
*   Open `http://54.179.175.198:8000` in your browser.
*   Click **Login**. It should redirect you to GitHub -> "Authorized AWS Woodpecker CI?" -> **Yes**.
*   You will see your repo list. Enable the crawler repo.
*   Done! pipeline is active.

### 3️⃣ The Pipeline (`.woodpecker.yml`)
Add this file to your repository root instead of `.github/workflows`.

```yaml
steps:
  build:
    image: rust:latest
    commands:
      - cargo build --release

  publish:
    image: plugins/docker
    settings:
      username: my_user
      password: my_password
      repo: my_registry/crawler
      tags: latest
```

### ✅ The Result
1.  You push code to **GitHub**.
2.  GitHub notifies **Woodpecker** (on your server).
3.  Woodpecker runs the build on your server.
4.  **GitHub Actions** is completely bypassed.
