# 🔐 Security Audit Report

## 1. Private SSH Keys (`*.pem`)
**Status: ✅ SAFE (Ignored)**
- `myanmar-vpn-key.pem` and `sg-crawling-key.pem` are present in your local folder but are **ignored** by git (listed in `.gitignore`).
- They have **never** been committed to the repository history.
- **Action**: Delete them from the local folder or move them to `~/.ssh/` to prevent accidental inclusion if `.gitignore` is ever broken.

## 2. Database Credentials
**Status: ⚠️ HIGH RISK**
- **Issue**: `docker-compose.prod.yml` contains a hardcoded password:
  ```yaml
  POSTGRES_PASSWORD: crawler_password
  ```
- **Exposure**: Port `5432` is mapped to the host (`"5432:5432"`). If your server IP is public, anyone can try to log in with this weak password.
- **Fix**:
  1. Remove `ports: - "5432:5432"` from `docker-compose.prod.yml` (the crawler connects internally, so external access isn't needed).
  2. Use an `.env` file for the password and do not commit it.

## 3. Other Findings
- No other obvious API keys (Google/Bing) were found hardcoded in the source code.
- `jobs.json` contains metadata about GitHub Actions but no secrets.

## Recommendations
1. **Immediate**: Restrict port 5432 on your firewall (AWS Security Groups / UFW).
2. **Cleanup**: Follow the file cleanup plan to remove clutter.
