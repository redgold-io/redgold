
# TODO: map to username / password / extract this
#curl -X POST -H "Content-Type: application/json" -d '{
#  "name":"automation-key",
#  "role": "Admin"
#}' http://admin:admin@localhost:3000/api/auth/keys


#curl -H "Authorization: Bearer $1" "http://$2:3000/api/v1/provisioning/alert-rules/$3"
curl -X POST -H 'Content-Type: application/json' -H "Authorization: Bearer $1" -H 'X-Disable-Provenance: true' -d @alerts.json http://$2:3000/api/v1/provisioning/alert-rules