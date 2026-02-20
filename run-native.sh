#!/bin/bash
set -e

# Add cargo to PATH
export PATH="$HOME/.cargo/bin:$PATH"

# Source cargo environment if it exists
[ -f "$HOME/.cargo/env" ] && source "$HOME/.cargo/env"

# Load environment variables from .env file
set -a
source <(cat .env | grep -v '^#' | grep -v '^$')
set +a

echo "🔨 Building release binary with fan control..."
cargo build --release

echo ""
echo "🚀 Starting govee2mqtt..."
echo "📡 Connecting to MQTT broker at $GOVEE_MQTT_HOST:$GOVEE_MQTT_PORT"
echo "🔍 Looking for your H7105 fan..."
echo ""
./target/release/govee serve
