#! /bin/bash

echo -n "Generate InfluxDB config file"

echo \
'{
	"url": "'${INFLUXDB_URL}'",
	"token": "'${INFLUXDB_TOKEN}'",
	"org": "'${INFLUXDB_ORG}'",
	"active": true,
	"verify_ssl": false
}' > config.json;