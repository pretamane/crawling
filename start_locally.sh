#!/bin/bash
set -e

echo "🚀 Starting Local Environment..."

# 1. Start Database and Adminer in background
echo "📦 Spinning up PostgreSQL and Adminer..."
docker-compose up -d db adminer

# 2. Wait for DB availability (simple sleep)
echo "⏳ Waiting for Database to be ready..."
sleep 5

# 3. Start Rust Crawler
echo "🦀 Starting Rust Crawler (Development Mode)..."
echo "👉 API: http://localhost:3000"
echo "👉 Adminer: http://localhost:8080"

cd rust-crawler
# Ensure .env is loaded if it exists
if [ -f .env ]; then
    echo "Loading .env file..."
    set -o allexport
    source .env
    set +o allexport
fi

cargo run
