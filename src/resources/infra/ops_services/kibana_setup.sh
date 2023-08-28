curl -X POST "http://localhost:5601/api/saved_objects/index-pattern/filebeat-7.8.0-*" \
     -H "kbn-xsrf: true" \
     -H "Content-Type: application/json" \
     -d '{
        "attributes": {
            "title": "filebeat-7.8.0-*",
            "timeFieldName": "@timestamp"
        }
     }'

curl -X POST "http://localhost:5601/api/kibana/settings/defaultIndex" \
     -H "kbn-xsrf: true" \
     -H "Content-Type: application/json" \
     -d '{
        "value": "filebeat-7.8.0-*"
     }'