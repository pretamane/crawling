# 📂 Repository Root File Audit

Analysis of "scattered" files in the root directory.

## 🚨 Critical Security Warnings

> [!CAUTION]
> **Private Keys Exposed**: `myanmar-vpn-key.pem` and `sg-crawling-key.pem` are private SSH keys located in the root of the repo.
> **Action Required**:
> 1. Ensure you have copies of these keys securely stored (e.g., `~/.ssh/`).
> 2. **DELETE** them from this directory immediately.
> 3. Add `*.pem` to your `.gitignore`.

## File Usage & Analysis

| File Name | Purpose | Recommendation |
|-----------|---------|----------------|
| `Caddyfile` | **Web Server Config**. Defines your reverse proxy rules for `thaw-zin-2k77.de5.net`. | **Keep**. Critical for production. |
| `docker-compose.yml` | **Orchestration**. Main definition for running the stack. | **Keep**. |
| `docker-compose.prod.yml` | **Orchestration**. Production overrides (restart policies, env vars). | **Keep**. |
| `deploy_with_local_build.sh` | **Utility**. Script to compile Rust locally and push binary to Docker. | **Move** to `scripts/`. |
| `start_locally.sh` | **Utility**. Shortcut to start the dev environment. | **Move** to `scripts/`. |
| `architecture.mermaid` | **Docs**. System design diagram. | **Move** to `docs/`. |
| `rust_crawler_api.postman_...` | **Testing**. Postman collection for API endpoints. | **Move** to `docs/` or `tests/`. |
| `jobs.json` | **Artifact**. Dump of a GitHub Actions run (likely for debugging). | **Delete**. No longer needed. |
| `rw-crawler.tar.gz` | **Artifact**. Massive build artifact (~350MB). | **Delete**. Don't store binaries in git. |
| `logs.zip` | **Artifact**. Old log dump. | **Delete**. |

## Proposed Cleanup Plan

If you agree, I can run a cleanup script to organize this repo.

```bash
# 1. Organize
mkdir -p scripts docs tests
mv *.sh scripts/
mv *.mermaid docs/
mv *.postman_collection.json docs/

# 2. Cleanup
rm rust-crawler.tar.gz logs.zip jobs.json

# 3. Security (Waiting for your approval)
# rm *.pem
```
