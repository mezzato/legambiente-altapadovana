#! /bin/bash

export DATE=$(/bin/date -d yesterday +%F)
export WORKPATH=$(readlink -f ./mirror)

export SENSORS_BY_CITY=$(readlink -f ./sensors_by_city.csv)

echo Date: $DATE

mkdir -p $WORKPATH

host archive.sensor.community
sleep 2

pushd $WORKPATH

if [ -f "$SENSORS_BY_CITY" ]; then
    echo "$SENSORS_BY_CITY found."
else
    echo "$SENSORS_BY_CITY not found. You need a $SENSORS_BY_CITY file."
    exit 1
fi


IFS=$'\n' # set the Internal Field Separator to newline
for line in $(sed 1,1d $SENSORS_BY_CITY); do
    IFS=$','
    split=($line)
    unset IFS

    echo $line
    # sensor_id,sensor_type,node,city
    # $split is now a bash array
    sensor_id=${split[0]}
    sensor_type=$(echo "${split[1]}" | sed 's/./\L&/g')

    file="${DATE}_${sensor_type}_sensor_${sensor_id}.csv"

    echo "Downloading $file"
    curl -O https://archive.sensor.community/$DATE/$file

    if [ ! -f "$file" ]; then
        echo "$file not found."
        exit 1
    fi

    sed -i 's/;/,/g' $file

    echo "Uploading $file to influxdb"
    influx write -b sensorcommunity \
        -f $file \
        --header "#constant measurement,particulate" \
        --header "#datatype tag,tag,tag,double,double,dateTime:2006-01-02T15:04:05,double,double,double,double,double,double" \
        --skip-verify

    rm $file

done

popd
