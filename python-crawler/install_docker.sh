#!/bin/bash
set -e

echo "Updating apt..."
sudo apt-get update -y

echo "Installing Docker..."
sudo apt-get install -y docker.io

echo "Starting Docker..."
sudo systemctl start docker
sudo systemctl enable docker

echo "Adding user to docker group..."
sudo usermod -aG docker ubuntu

echo "Docker installation complete!"
