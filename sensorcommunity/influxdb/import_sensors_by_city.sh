#! /bin/bash

export WORKPATH=$(readlink -f ./mirror)

export SENSORS_BY_CITY=$(readlink -f ./sensors_by_city.csv)

if [ -f "$SENSORS_BY_CITY" ]; then
    echo "$SENSORS_BY_CITY found."
else
    echo "$SENSORS_BY_CITY not found. You need a $SENSORS_BY_CITY file."
    exit 1
fi

echo "Uploading $file to influxdb"
influx write -b sensorcommunity \
    -f $file \
    --header "#constant measurement,sensors_by_city" \
    --header "#datatype tag,tag,tag,tag" \
    --skip-verify

