# 🔄 Zero-Budget CI/CD Alternatives to GitHub
## (Self-Hosted & Private)

If you want to move away from GitHub but keep the "Push-to-Deploy" magic, here are the best tools that fit your **Zero-Budget / Privacy-First** capability.

---

### 1. 🏆 Top Pick: Gitea + Gitea Actions
**Why it's perfect for you:**
*   **Lightweight**: Runs on a Raspberry Pi or your AWS free tier (alongside the crawler).
*   **Familiar**: Looks exactly like GitHub.
*   **Compatible**: "Gitea Actions" are almost identical to GitHub Actions. You can reuse your `.github/workflows` YAML files!
*   **Cost**: $0 (Free & Open Source).

**Workflow:**
1.  Install Gitea on your AWS Server (Docker container).
2.  Push code to your *Private* Gitea URL (e.g., `git.thaw-zin-2k77.de5.net`).
3.  Gitea Runner (deployed on your Server or Laptop) sees the push and builds/deploys the crawler.

---

### 2. 🦜 Woodpecker CI
**Philosophy**: "Simple & Container-based".
*   **Integration**: Works great with Gitea.
*   **Architecture**: Every step runs inside a Docker container.
*   **Config**: Uses a simple `.woodpecker.yml` file.
*   **Scenario**: If you want a super-clean, minimal CI pipeline that just "runs docker build", this is it.

---

### 3. 🦊 Gitlab (Self-Hosted)
**Verdict**: **Avoid** for this project.
*   **Reason**: It consumes MASSIVE resources (4GB+ RAM just to idle). It will crash your free-tier AWS instance. Only use if you have a dedicated server.

---

### 4. 🤵 Jenkins
**Verdict**: **Too Complex**.
*   **Reason**: It's the "Swiss Army Knife" of CI, but requires heavy Java, lots of plugins, and maintenance. Overkill for a single laptop/crawler setup.

---

### 🚀 Recommendation: The "Shadow GitHub" Setup

**Install Gitea on your AWS Server.**
1.  It gives you a private Git repo (Safe from DMCA/Takedowns).
2.  It gives you CI/CD (Gitea Actions).
3.  It keeps your code 100% under your control.

**Docker Compose Snippet for Gitea:**
```yaml
services:
  gitea:
    image: gitea/gitea:latest
    ports:
      - "3000:3000"
      - "2222:22"
    volumes:
      - gitea_data:/data
```
