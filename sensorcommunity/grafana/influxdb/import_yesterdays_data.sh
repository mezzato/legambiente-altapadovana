#! /bin/bash

export DATE=$(/bin/date -d yesterday +%F)
export WORKPATH=$(readlink -f ./mirror)

echo Datum: $DATE

mkdir -p $WORKPATH

host archive.sensor.community
sleep 10

pushd $WORKPATH

SENSORS_BY_CITY=sensors_by_city.csv

if [ -f "$SENSORS_BY_CITY" ]; then
    echo "$SENSORS_BY_CITY found."
else
    echo "$SENSORS_BY_CITY not found. You need a $SENSORS_BY_CITY file."
    exit 1
fi

for line in $(cat $SENSORS_BY_CITY); do
    IFS=$','
    split=($line)
    unset IFS
    # sensor_id,sensor_type,node,city
    # $split is now a bash array
    sensor_id=${split[0]}
    sensor_type=$(echo "${split[1]}" | sed 's/./\L&/g')
    # node=${split[2]}
    # city=${split[3]}
    file="${DATE}_${sensor_type}_sensor_${sensor_id}.csv"

    echo "Downloading $file"
    curl -O https://archive.sensor.community/$DATE/$file

    if [ -f "$file" ]; then
        echo "$file found."
    else
        echo "$file not found."
        exit 1
    fi

    sed 's/;/,/g' $file >tmp_$file

    echo "Uploading $file to influxdb"
    influx write -b sensorcommunity \
        -f tmp_$file \
        --header "#constant measurement,particulate" \
        --header "#datatype tag,tag,tag,double,double,dateTime:2006-01-02T15:04:05,double,double,double,double,double,double" \
        --skip-verify

    rm tmp_$file
    rm $file

done

popd
