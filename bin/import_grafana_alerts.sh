#!/bin/bash

# Check if hostname is provided
if [ "$#" -lt 2 ]; then
    echo "Usage: $0 <hostname> <admin_user> <admin_password>"
    echo "Example: $0 localhost admin admin"
    exit 1
fi

HOSTNAME=$1
ADMIN_USER=$2
ADMIN_PASSWORD=$3
GRAFANA_URL="http://${HOSTNAME}:3000"

# First check if service account exists and get its ID
echo "Checking for existing service account..."
EXISTING_SA=$(curl -s \
    -u "${ADMIN_USER}:${ADMIN_PASSWORD}" \
    "${GRAFANA_URL}/api/serviceaccounts/search?query=automation-account")

SA_ID=$(echo "${EXISTING_SA}" | grep -o '"id":[0-9]*' | grep -o '[0-9]*' | head -n1)

if [ ! -z "$SA_ID" ]; then
    echo "Found existing service account with ID: ${SA_ID}"
    # Delete existing service account
    echo "Deleting existing service account..."
    curl -X DELETE \
        -u "${ADMIN_USER}:${ADMIN_PASSWORD}" \
        "${GRAFANA_URL}/api/serviceaccounts/${SA_ID}"
fi

# Create new service account
echo "Creating service account..."
SA_RESPONSE=$(curl -X POST \
    -H "Content-Type: application/json" \
    -u "${ADMIN_USER}:${ADMIN_PASSWORD}" \
    -d '{"name":"automation-account","role": "Admin"}' \
    "${GRAFANA_URL}/api/serviceaccounts" 2>&1)

echo "Service account response: ${SA_RESPONSE}"

# Extract the service account ID
SA_ID=$(echo "${SA_RESPONSE}" | grep -o '"id":[0-9]*' | grep -o '[0-9]*')

if [ -z "$SA_ID" ]; then
    echo "Failed to create service account"
    exit 1
fi

# Create token for the service account
echo "Creating service account token..."
TOKEN_RESPONSE=$(curl -X POST \
    -H "Content-Type: application/json" \
    -u "${ADMIN_USER}:${ADMIN_PASSWORD}" \
    -d '{"name":"automation-token"}' \
    "${GRAFANA_URL}/api/serviceaccounts/${SA_ID}/tokens" 2>&1)

echo "Token response: ${TOKEN_RESPONSE}"

# Extract the token
API_KEY=$(echo "${TOKEN_RESPONSE}" | grep -o '"key":"[^"]*' | grep -o '[^"]*$')

if [ -z "$API_KEY" ]; then
    echo "Failed to create service account token"
    exit 1
fi

echo "Service account and token created successfully"

# Create folder for alerts
echo "Creating alerts folder..."
curl -X POST \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer ${API_KEY}" \
    -d '{
        "uid": "NH3iWvuVk",
        "title": "Redgold Alerts"
    }' \
    "${GRAFANA_URL}/api/folders"

# Import alert rules
echo "Importing alert rules..."
curl -X POST \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer ${API_KEY}" \
    -H "X-Disable-Provenance: true" \
    -d @src/resources/infra/ops_services/grafana/alerts.json \
    "${GRAFANA_URL}/api/v1/provisioning/alert-rules"

# Create notification folder
echo "Creating notification folder..."
curl -X POST \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer ${API_KEY}" \
    -d '{
        "title": "Alert Notifications"
    }' \
    "${GRAFANA_URL}/api/folders"

# Import notification templates and email settings
echo "Importing notification templates and email settings..."
curl -X POST \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer ${API_KEY}" \
    -H "X-Disable-Provenance: true" \
    -d @src/resources/infra/ops_services/grafana/email-template.json \
    "${GRAFANA_URL}/api/v1/provisioning/notification-policies"

echo "Import completed"