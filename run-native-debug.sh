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

# Set cache directory
export XDG_CACHE_HOME="$(pwd)/data"

# Enable debug logging including BLE packets
export RUST_LOG=govee=debug,govee::ble=trace,govee::service::iot=trace

echo "🔨 Building release binary with fan control..."
cargo build --release

echo ""
echo "🚀 Starting govee2mqtt with DEBUG logging..."
echo "📡 Connecting to MQTT broker at $GOVEE_MQTT_HOST:$GOVEE_MQTT_PORT"
echo "📁 Data directory: $XDG_CACHE_HOME"
echo ""
./target/release/govee serve \
  --govee-iot-key=./data/iot.key \
  --govee-iot-cert=./data/iot.cert \
  --amazon-root-ca=./AmazonRootCA1.pem
