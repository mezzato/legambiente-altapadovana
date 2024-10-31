#! /bin/bash

echo -n "start date (for example 2024-10-27): "
read START
echo -n "end date (for example 2024-10-27): "
read END

export WORKPATH=$(readlink -f ./mirror)
export SENSORS_BY_CITY=$(readlink -f ./sensors_by_city.csv)

mkdir -p $WORKPATH

pushd $WORKPATH

if [ -f "$SENSORS_BY_CITY" ]; then
    echo "$SENSORS_BY_CITY found."
else
    echo "$SENSORS_BY_CITY not found. You need a $SENSORS_BY_CITY file."
    exit 1
fi

host archive.sensor.community
sleep 1

echo start date: $START
echo end date: $END


# After this, startdate and enddate will be valid ISO 8601 dates,
# or the script will have aborted when it encountered unparseable data
# such as input_end=abcd
startdate=$(date -I -d "$START") || exit -1
enddate=$(date -I -d "$END") || exit -1

DATE="$startdate"
while [[ "$(date -d "$DATE" +%Y%m%d)" -le "$(date -d "$enddate" +%Y%m%d)" ]]; do
    echo "importing date: $DATE"

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
        chip_id=${split[2]}
        city=${split[3]}

        file="${DATE}_${sensor_type}_sensor_${sensor_id}.csv"

        echo "Downloading $file"
        curl -O https://archive.sensor.community/$DATE/$file

        if [ ! -f "$file" ]; then
            echo "$file not found."
            exit 1
        fi

        sed -i 's/;/,/g' $file

        # sensor_id;sensor_type;location;lat;lon;timestamp;P1;durP1;ratioP1;P2;durP2;ratioP2
        # 62574;SDS011;77629;45.630;11.704;2024-10-30T00:01:08;15.30;;;8.98;;
        echo "Uploading $file to influxdb"
        influx write -b sensorcommunity \
            -f $file \
            --header "#constant measurement,particulate" \
            --header "#constant tag,chip_id,${chip_id}" \
            --header "#constant tag,city,${city}" \
            --header "#datatype tag,tag,tag,double,double,dateTime:2006-01-02T15:04:05,double,double,double,double,double,double" \
            --skip-verify

        rm $file

    done

    DATE=$(date -I -d "$DATE + 1 day")
done

popd
