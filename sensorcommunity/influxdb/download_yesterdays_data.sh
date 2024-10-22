#! /bin/bash

export DATE=$(/bin/date -d yesterday +%F)
export WORKPATH=$(readlink -f ./mirror)

export SENSORS_BY_CITY=$(readlink -f ./sensors_by_city.csv)

echo Datum: $DATE

mkdir -p $WORKPATH

host archive.sensor.community
sleep 10

pushd $WORKPATH

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

done

popd
