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

export RUST_LOG=govee::service::iot=trace,govee::service::state=debug,govee::service::hass=trace,govee::hass_mqtt::fan=trace

echo "🔨 Building release binary with fan control..."
cargo build --release

echo ""
echo "🚀 Starting govee2mqtt..."
echo ""
./target/release/govee serve \
  --govee-iot-key=./data/iot.key \
  --govee-iot-cert=./data/iot.cert \
  --amazon-root-ca=./AmazonRootCA1.pem 2>&1 | grep --line-buffered "H7105\|ptReal\|multiSync\|Fan state\|Fan osc\|oscillat\|set.*speed\|notify-oscillat\|notify_of_state"
