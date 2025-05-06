## Set up user and permissions

```bash
sudo useradd -m influxdb
sudo mkdir -p /var/lib/influxdb3/
sudo chown -R influxdb:influxdb /var/lib/influxdb3/
sudo chmod 755 /var/lib/influxdb3/
```

## Install and deploy executable

see [installation steps](https://docs.influxdata.com/influxdb3/core/get-started/)

## Create configuration file

```bash
sudo vim /etc/default/influxdb3
sudo chmod a+r /etc/default/influxdb3
```

Add for instance:

```bash
INFLUXDB3_OBJECT_STORE=file
INFLUXDB3_DB_DIR=~/.influxdb3
LOG_FILTER=info
INFLUXDB3_MAX_HTTP_REQUEST_SIZE=20971520
INFLUXDB3_BEARER_TOKEN=mytoken
INFLUXDB3_NODE_IDENTIFIER_PREFIX=host01
INFLUXDB3_HTTP_BIND_ADDR=0.0.0.0:8181
```

## Deploy systemd unit

```bash
sudo cp .influxdb/influxdb3 /usr/bin/
sudo cp influxdb.service /etc/systemd/system
sudo systemctl enable influxdb.service
sudo systemctl daemon-reload
echo start the service
sudo systemctl start influxdb.service
```

## Create the database

```bash
export INFLUXDB3_AUTH_TOKEN=[my token]
./influxdb3 create database sensorcommunity
```