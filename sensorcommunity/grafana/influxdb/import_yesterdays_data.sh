#! /bin/bash

export DATE=`/bin/date -d yesterday +%F`
export WORKPATH=$(readlink -f ./mirror)

echo Datum: $DATE

mkdir -p $WORKPATH

host archive.sensor.community
sleep 10

pushd $WORKPATH

file="${DATE}_sds011_sensor_62574.csv"

echo "Downloading $file"
curl -O https://archive.sensor.community/$DATE/$file

if [ -f "$file" ]; then
    echo "$file found."
else
    echo "$file not found. You need to build the executable first."
    exit 1
fi

sed 's/;/,/g' $file >tmp_$file
# cat <(echo "sep=;") $file > tmp_$1

echo "Uploading $file to influxdb"
influx write -b sensorcommunity \
    -f tmp_$file \
    --header "#constant measurement,particulate" \
    --header "#datatype tag,tag,tag,double,double,dateTime:2006-01-02T15:04:05,double,double,double,double,double,double" \
    --skip-verify

rm tmp_$file
rm $file

popd