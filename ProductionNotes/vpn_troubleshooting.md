# 🩺 VPN Troubleshooting: Bypassing "MaharNet" Throttling

Your WiFi (MaharNet) seems to be performing deeper analysis or has strict QoS than your mobile data. Here is how to diagnose and bypass it.

## 1. The MTU Problem (Likely Culprit) 📦
VPNs add "headers" to packets, making them larger. If a packet exceeds the network's limit (MTU), it gets dropped, causing "slowness" or "hanging" connections.

**Test:**
In your V2Ray client (v2rayNG / V2Box), look for **MTU** setting in the settings or server config.
- **Current Default**: Usually `1500` or `1600`
- **Change to**: `1280`

**Why?** `1280` is the safe minimum. If this fixes it, your ISP has a low MTU for encrypted UDP/TCP traffic.

## 2. SNI Blocking (The Disguise) 🎭
You are mimicking `www.google.com`. If MaharNet blocks Google or checks IPs strictly, this fails.

**Try changing the SNI** in your client and server config.
Good alternatives for AWS Singapore:
- `s3.ap-southeast-1.amazonaws.com` (AWS native - looks very legitimate going to your IP)
- `www.amazon.com`
- `www.microsoft.com`

*Note: You must update `config.json` on the server and `dest` in your client to match.*

## 3. The "Fingerprint" 🕵️‍♂️
Your client is set to `fp: chrome`.
- Try changing **uTLS / Fingerprint** in v2rayNG to `randomized` or `ios`.
- Fixed fingerprints can sometimes be flagged by advanced DPI.

## 4. IP Throttling 🛑
If they are throttling `54.179.175.198` directly (regardless of protocol), no setting will fix it.
**Diagnosis**:
- Turn OFF VPN.
- Run `ping 54.179.175.198`.
- If ping is high/lossy without VPN, your ISP hates this specific IP. You might need a new AWS IP (Stop/Start instance).
