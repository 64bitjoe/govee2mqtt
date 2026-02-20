#!/bin/bash
set -e

echo "🔨 Building Docker image with fan control support..."
docker compose -f docker-compose.dev.yml build

echo ""
echo "🚀 Starting govee2mqtt container..."
docker compose -f docker-compose.dev.yml up -d

echo ""
echo "✅ Container started!"
echo ""
echo "📋 View logs with:"
echo "   docker logs govee2mqtt-dev --follow"
echo ""
echo "🛑 Stop with:"
echo "   docker compose -f docker-compose.dev.yml down"
