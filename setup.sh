#!/usr/bin/env bash
set -euo pipefail

# Zap - Local infrastructure setup
# Sets up Tuwunel (Matrix homeserver) + mautrix-meta (Messenger bridge)

BOLD='\033[1m'
DIM='\033[2m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m'

step() { echo -e "\n${BOLD}${GREEN}==>${NC} ${BOLD}$1${NC}"; }
info() { echo -e "  ${CYAN}$1${NC}"; }
warn() { echo -e "  ${YELLOW}$1${NC}"; }
err()  { echo -e "  ${RED}$1${NC}"; }

# Fix ownership of bridge data files (container runs as root)
fix_bridge_perms() {
    if [ -d docker/mautrix-meta ]; then
        docker run --rm -v "$(pwd)/docker/mautrix-meta:/data" alpine \
            sh -c "chown -R $(id -u):$(id -g) /data && chmod -R u+rw /data" 2>/dev/null || true
    fi
}

# Check prerequisites
command -v docker >/dev/null 2>&1 || { err "Docker is not installed."; exit 1; }
command -v docker compose >/dev/null 2>&1 || { err "Docker Compose is not installed."; exit 1; }
command -v curl >/dev/null 2>&1 || { err "curl is not installed."; exit 1; }

cd "$(dirname "$0")"

# ──────────────────────────────────────────────────────────
# Step 1: Start homeserver
# ──────────────────────────────────────────────────────────
step "Step 1: Starting Tuwunel homeserver..."
docker compose up -d homeserver
sleep 3

# Verify homeserver is running
if curl -sf http://localhost:6167/_matrix/client/versions > /dev/null 2>&1; then
    info "Homeserver is running at http://localhost:6167"
else
    warn "Homeserver may still be starting up. Give it a few seconds."
fi

# ──────────────────────────────────────────────────────────
# Step 2: Create Matrix account
# ──────────────────────────────────────────────────────────
step "Step 2: Create your Matrix account"
echo ""
read -rp "  Choose a username (e.g. tdinh): " MATRIX_USER
read -rsp "  Choose a password: " MATRIX_PASS
echo ""

# Register via the Matrix client API
REGISTER_RESPONSE=$(curl -sf -X POST "http://localhost:6167/_matrix/client/v3/register" \
    -H "Content-Type: application/json" \
    -d "{
        \"username\": \"${MATRIX_USER}\",
        \"password\": \"${MATRIX_PASS}\",
        \"auth\": {
            \"type\": \"m.login.registration_token\",
            \"token\": \"${REGISTRATION_TOKEN:-zap-setup-token}\"
        },
        \"initial_device_display_name\": \"Zap Setup\"
    }" 2>&1) || true

if echo "$REGISTER_RESPONSE" | grep -q "user_id"; then
    MATRIX_USER_ID=$(echo "$REGISTER_RESPONSE" | grep -o '"user_id":"[^"]*"' | cut -d'"' -f4)
    info "Account created: ${MATRIX_USER_ID}"
    info "This is the first account, so it has admin privileges."
else
    warn "Registration response: ${REGISTER_RESPONSE}"
    warn "If the account already exists, that's fine. Continuing..."
    MATRIX_USER_ID="@${MATRIX_USER}:localhost"
fi

# Get an access token for admin operations
step "Step 2b: Getting admin access token..."
LOGIN_RESPONSE=$(curl -sf -X POST "http://localhost:6167/_matrix/client/v3/login" \
    -H "Content-Type: application/json" \
    -d "{
        \"type\": \"m.login.password\",
        \"identifier\": {\"type\": \"m.id.user\", \"user\": \"${MATRIX_USER}\"},
        \"password\": \"${MATRIX_PASS}\",
        \"initial_device_display_name\": \"Zap Setup\"
    }" 2>&1) || true

ACCESS_TOKEN=""
if echo "$LOGIN_RESPONSE" | grep -q "access_token"; then
    ACCESS_TOKEN=$(echo "$LOGIN_RESPONSE" | grep -o '"access_token":"[^"]*"' | cut -d'"' -f4)
    info "Logged in successfully."
else
    warn "Could not get access token. Appservice registration will need to be done manually."
fi

# ──────────────────────────────────────────────────────────
# Step 3: Generate mautrix-meta config
# ──────────────────────────────────────────────────────────
step "Step 3: Setting up mautrix-meta bridge..."

if [ ! -f docker/mautrix-meta/config.yaml ]; then
    info "First run: generating default config..."
    docker compose run --rm meta-bridge || true
    sleep 2
    fix_bridge_perms
fi

if [ -f docker/mautrix-meta/config.yaml ]; then
    info "Configuring bridge to connect to homeserver..."

    # Patch homeserver address and domain
    sed -i "s|address: http://example.localhost:8008|address: http://homeserver:6167|g" docker/mautrix-meta/config.yaml
    sed -i "s|domain: example.com|domain: localhost|g" docker/mautrix-meta/config.yaml

    # Patch appservice address and hostname for Docker networking
    sed -i "s|address: http://localhost:29319|address: http://meta-bridge:29319|g" docker/mautrix-meta/config.yaml
    sed -i "s|hostname: 127.0.0.1|hostname: 0.0.0.0|g" docker/mautrix-meta/config.yaml

    # Switch database to SQLite
    sed -i "s|type: postgres|type: sqlite3-fk-wal|g" docker/mautrix-meta/config.yaml
    sed -i "s|uri: postgres://user:password@host/database?sslmode=disable|uri: file:///data/mautrix-meta.db?_txlock=immediate|g" docker/mautrix-meta/config.yaml

    # Set bridge permissions
    sed -i 's|"example.com": user|"localhost": user|g' docker/mautrix-meta/config.yaml
    sed -i "s|\"@admin:example.com\": admin|\"${MATRIX_USER_ID}\": admin|g" docker/mautrix-meta/config.yaml

    # Set mode to messenger (only patch the empty mode line, not other mode references)
    sed -i 's|^    mode:$|    mode: messenger|g' docker/mautrix-meta/config.yaml

    info "Config updated."
else
    err "Failed to generate config. Check docker logs."
    exit 1
fi

# ──────────────────────────────────────────────────────────
# Step 4: Generate registration file
# ──────────────────────────────────────────────────────────
step "Step 4: Generating bridge registration..."

if [ ! -f docker/mautrix-meta/registration.yaml ]; then
    docker compose run --rm meta-bridge || true
    sleep 2
    fix_bridge_perms
fi

if [ -f docker/mautrix-meta/registration.yaml ]; then
    info "Registration file generated."
else
    err "Failed to generate registration.yaml"
    exit 1
fi

# ──────────────────────────────────────────────────────────
# Step 5: Register appservice with homeserver
# ──────────────────────────────────────────────────────────
step "Step 5: Registering bridge with homeserver..."

REGISTRATION_YAML=$(cat docker/mautrix-meta/registration.yaml)

if [ -n "$ACCESS_TOKEN" ]; then
    info "Attempting to register appservice via admin API..."

    # Join the admin room first
    curl -sf -X POST "http://localhost:6167/_matrix/client/v3/join/%23admins:localhost" \
        -H "Authorization: Bearer ${ACCESS_TOKEN}" \
        -H "Content-Type: application/json" \
        -d '{}' > /dev/null 2>&1 || true

    # Find the admin room ID
    ROOMS_RESPONSE=$(curl -sf "http://localhost:6167/_matrix/client/v3/joined_rooms" \
        -H "Authorization: Bearer ${ACCESS_TOKEN}" 2>&1) || true

    ADMIN_ROOM_ID=""
    if echo "$ROOMS_RESPONSE" | grep -q "joined_rooms"; then
        # Get all room IDs and find the admin room
        ROOM_IDS=$(echo "$ROOMS_RESPONSE" | grep -o '"![^"]*"' | tr -d '"')
        for ROOM_ID in $ROOM_IDS; do
            ROOM_STATE=$(curl -sf "http://localhost:6167/_matrix/client/v3/rooms/$(echo "$ROOM_ID" | sed 's/!/%21/g; s/:/%3A/g')/state/m.room.name" \
                -H "Authorization: Bearer ${ACCESS_TOKEN}" 2>&1) || true
            if echo "$ROOM_STATE" | grep -qi "admin"; then
                ADMIN_ROOM_ID="$ROOM_ID"
                break
            fi
        done
    fi

    if [ -n "$ADMIN_ROOM_ID" ]; then
        ENCODED_ROOM=$(echo "$ADMIN_ROOM_ID" | python3 -c "import sys,urllib.parse; print(urllib.parse.quote(sys.stdin.read().strip()))")

        # Send register_appservice command with the YAML content
        BODY=$(printf '{"msgtype":"m.text","body":"register_appservice\\n\\n```yaml\\n%s\\n```"}' "$(echo "$REGISTRATION_YAML" | sed 's/"/\\"/g' | sed ':a;N;$!ba;s/\n/\\n/g')")

        SEND_RESPONSE=$(curl -sf -X PUT \
            "http://localhost:6167/_matrix/client/v3/rooms/${ENCODED_ROOM}/send/m.room.message/$(date +%s%N)" \
            -H "Authorization: Bearer ${ACCESS_TOKEN}" \
            -H "Content-Type: application/json" \
            -d "$BODY" 2>&1) || true

        if echo "$SEND_RESPONSE" | grep -q "event_id"; then
            info "Appservice registration command sent to admin room."
            sleep 2
            warn "Check the admin room for confirmation. If it failed, register manually."
        else
            warn "Could not send to admin room. You'll need to register manually."
        fi
    else
        warn "Could not find admin room. You'll need to register manually."
    fi
else
    warn "No access token available. Manual registration required."
fi

echo ""
info "If manual registration is needed:"
info "  1. Open Element (https://app.element.io)"
info "  2. Log in: ${MATRIX_USER_ID} @ http://localhost:6167"
info "  3. Join #admins:localhost"
info "  4. Send: register_appservice"
info "  5. Paste registration.yaml contents in a code block"

# ──────────────────────────────────────────────────────────
# Step 6: Start the bridge
# ──────────────────────────────────────────────────────────
step "Step 6: Starting the bridge..."
docker compose up -d meta-bridge
sleep 3

if docker compose ps meta-bridge 2>/dev/null | grep -q "Up\|running"; then
    info "Bridge is running."
else
    warn "Bridge may have issues. Check: docker compose logs meta-bridge"
fi

# ──────────────────────────────────────────────────────────
# Step 7: Update Zap config
# ──────────────────────────────────────────────────────────
step "Step 7: Configuring Zap..."

ZAP_CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/zap"
mkdir -p "$ZAP_CONFIG_DIR"

if [ -f "$ZAP_CONFIG_DIR/config.toml" ]; then
    sed -i "s|username = \".*\"|username = \"${MATRIX_USER_ID}\"|g" "$ZAP_CONFIG_DIR/config.toml"
    info "Updated username in $ZAP_CONFIG_DIR/config.toml"
else
    cat > "$ZAP_CONFIG_DIR/config.toml" << EOF
[matrix]
homeserver = "http://localhost:6167"
username = "${MATRIX_USER_ID}"

[ui]
theme = "default"
room_list_width = 30
timestamp_format = "%H:%M"
show_help_bar = true

[behavior]
vim_mode = true
send_read_receipts = true
EOF
    info "Created $ZAP_CONFIG_DIR/config.toml"
fi

# ──────────────────────────────────────────────────────────
# Step 8: Login to Meta Messenger
# ──────────────────────────────────────────────────────────
step "Step 8: Connect to Meta Messenger"
echo ""
info "Once Zap or Element is connected, DM the bridge bot: @metabot:localhost"
info "Send: login"
info "The bot will ask you to extract cookies from messenger.com"
echo ""
info "To extract cookies:"
info "  1. Open messenger.com in an incognito/private browser"
info "  2. Log in to your Meta account"
info "  3. Open DevTools (F12) > Network tab"
info "  4. Find any GraphQL request"
info "  5. Right-click > Copy as cURL"
info "  6. Paste the cURL command to the bridge bot"
echo ""

# ──────────────────────────────────────────────────────────
# Done
# ──────────────────────────────────────────────────────────
step "Setup complete!"
echo ""
info "Start everything:    docker compose up -d"
info "Stop everything:     docker compose down"
info "View logs:           docker compose logs -f"
info "Run Zap:             cargo run"
info "Run Zap (offline):   cargo run -- --offline"
echo ""
