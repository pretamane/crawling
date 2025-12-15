# 🌐 Zero-Budget Distributed Crawler Setup
## (Using Your ER605 Router + Laptop Farm)

This guide turns your home network into a **Distributed Residential Proxy Farm** using your **TP-Link ER605 v2.2** and existing laptops/PCs.

---

### 🏛️ The Architecture
*   **The Brain (AWS Singapore)**: Stores the Database, Queue, and Files (MinIO).
*   **The Body (Your Home)**: Executes the scrapes using Residential IPs.
*   **The Router (ER605)**: Routes different workers through different ISPs.

---

### 1️⃣ Network Preparation (Virtual IPs)
Instead of buying 3 computers, give your Linux laptop 3 extra IP addresses (Aliases).

**Command (Run on Laptop):**
```bash
# Add 3 Virtual IPs to your main interface (e.g., eth0 or wlan0)
sudo ip addr add 192.168.0.101/24 dev wlan0
sudo ip addr add 192.168.0.102/24 dev wlan0
sudo ip addr add 192.168.0.103/24 dev wlan0
```
*Verify with `ip addr show`.*

---

### 2️⃣ ER605 Router Configuration (The Magic) 🪄
We will use **Policy Routing** to force each IP to use a different ISP.

1.  **Login** to ER605 (usually `192.168.0.1`).
2.  **Create IP Groups**:
    *   Go to `Preferences` -> `IP Group`.
    *   Create Group `Worker_A` -> Address: `192.168.0.101`.
    *   Create Group `Worker_B` -> Address: `192.168.0.102`.
    *   Create Group `Worker_C` -> Address: `192.168.0.103`.
3.  **Set Policy Routing**:
    *   Go to `Transmission` -> `Routing` -> `Policy Routing`.
    *   **Rule A**: Source=`Worker_A` ➡️ WAN=`WAN1` (ISP 1).
    *   **Rule B**: Source=`Worker_B` ➡️ WAN=`WAN2` (ISP 2).
    *   **Rule C**: Source=`Worker_C` ➡️ WAN=`WAN3` (ISP 3).

*Now, any traffic from `192.168.0.101` will AUTOMATICALLY exit via ISP 1.*

---

### 3️⃣ Connect to The Brain (SSH Tunnel) 🚇
The workers need to talk to the AWS Redis/DB. We tunnel these ports securely.

**Command (Run on Laptop):**
```bash
# Map AWS Redis(6379), DB(5432), MinIO(9000) to Local Ports
ssh -i sg-crawling-key.pem -N -o StrictHostKeyChecking=no -L 6380:localhost:6379 -L 5433:localhost:5432 -L 9005:localhost:9000 ubuntu@54.179.175.198
```
*(Keep this running in a background terminal)*

---

### 4️⃣ Launch the Swarm 🐝
Run 3 independent workers, each bound to a specific source IP.

**Worker A (ISP 1):**
```bash
export BIND_Address=192.168.0.101  # (Requires enabling binding in code)
export REDIS_URL=redis://127.0.0.1:6380
export DATABASE_URL=postgres://crawler:crawler_password@127.0.0.1:5433/crawling_db
export MINIO_ENDPOINT=http://127.0.0.1:9005
./rust-crawler
```

### Summary
1.  Laptop creates 3 IPs.
2.  Router sends IPs to 3 ISPs.
3.  Workers use IPs to scrape.
4.  Data flows back to AWS via Tunnel.
