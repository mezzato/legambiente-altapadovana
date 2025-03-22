#!/bin/bash -e

# /home/dataingester/.influxdb/influxdb3' serve --node-id='host01' --http-bind='0.0.0.0:8181' --object-store=file --data-dir /home/dataingester/.influxdb/data

# /usr/lib/influxdb/scripts/influxd-systemd-start.sh

WORKDIR=~/.influxdb
${WORKDIR}/influxdb3 serve \
  --http-bind '0.0.0.0:8181' \
  --object-store file \
  --data-dir ${WORKDIR}/data \
  --node-id 'host01' \
  --log-filter info \
  --max-http-request-size 20971520 &

PID=$!
echo $PID > /var/lib/influxdb3/influxd.pid

echo "InfluxDB started"
