#!/bin/bash
set -e

SERVER_IP="54.179.175.198"
KEY_FILE="/home/guest/.ssh/crawling_keys/sg-crawling-key.pem"

echo "🚀 Starting Local Build & Deploy..."

# 1. Build Local Image
echo "🔨 Building Docker image locally..."
docker build -t rust-crawler:latest ./rust-crawler

# 2. Upload Image to Server
echo "📤 Uploading image to $SERVER_IP (this might take a while)..."

# Save to file first
echo "💾 Saving image to local file..."
docker save rust-crawler:latest | gzip > rust-crawler.tar.gz

# Upload via SCP
echo "📤 Uploading image file to $SERVER_IP..."
scp -i $KEY_FILE rust-crawler.tar.gz ubuntu@$SERVER_IP:~/rust-crawler.tar.gz

# Load on server
echo "📥 Loading image on server..."
ssh -i $KEY_FILE ubuntu@$SERVER_IP "gunzip -c rust-crawler.tar.gz | sudo docker load && sudo docker tag localhost/rust-crawler:latest rust-crawler:latest && rm rust-crawler.tar.gz"


# 3. Upload Configuration
echo "⚙️ Uploading production config..."
scp -i $KEY_FILE docker-compose.prod.yml ubuntu@$SERVER_IP:~/crawling/docker-compose.prod.yml

# 4. Restart Services
echo "🔄 Restarting services..."
ssh -i $KEY_FILE ubuntu@$SERVER_IP << 'EOF'
    cd crawling
    sudo docker-compose -f docker-compose.prod.yml down
    sudo docker-compose -f docker-compose.prod.yml up -d
EOF

echo "✅ Deployment Complete! Check http://$SERVER_IP:3000"
