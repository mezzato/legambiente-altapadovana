#! /bin/bash

echo -n "Install influx client"

wget -q https://dl.influxdata.com/influxdb/releases/influxdb2-client-2.7.5-linux-amd64.tar.gz
mkdir -p ./bin
tar -C ./bin -xzf ./influxdb2-client-2.7.5-linux-amd64.tar.gz

cd ./bin

echo -n "Generate InfluxDB config file"

./influx config create --config-name legambiente \
  --host-url ${INFLUXDB_URL} \
  --org ${INFLUXDB_ORG} \
  --token ${INFLUXDB_TOKEN} \
  --active
