#! /bin/bash

export SENSORS_BY_CITY=$(readlink -f ./sensors_by_city.csv)

if [ -f "$SENSORS_BY_CITY" ]; then
    echo "$SENSORS_BY_CITY found."
else
    echo "$SENSORS_BY_CITY not found. You need a $SENSORS_BY_CITY SENSORS_BY_CITY."
    exit 1
fi


echo "Uploading $SENSORS_BY_CITY to influxdb"
influx write -b sensorcommunity \
    -f $SENSORS_BY_CITY \
    --header "#constant measurement,sensors_by_city" \
    --header "#datatype tag,tag,tag,tag" \
    --skip-verify

