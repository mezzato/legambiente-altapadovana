#! /bin/bash

# variable is not set at all
if [[ ! ${START+x} ]]; then
  echo -n "start date (for example 2024-10-27), default is yesterday: "
  read START
fi

if [[ ! ${END+x} ]]; then
  echo -n "end date (for example 2024-10-27), default is yesterday: "
  read END
fi

# variable resolves to an empty string
if [[ ! $START ]]; then
  START="$(date -d "yesterday" +%Y%m%d)"
fi

# variable resolves to an empty string
if [[ ! $END ]]; then
  END="$(date -d "yesterday" +%Y%m%d)"
fi

echo "---------- $SENSORS --------"
if [[ ! ${SENSORS+x} ]]; then
  echo -n "no sensors specified\n"
  SENSORS=""
fi

if [[ ! ${CSV_FILE+x} ]]; then
  echo -n "csv file with sensors_by_city (default ./sensors_by_city.csv): "
  read CSV_FILE
fi

# variable resolves to an empty string
if [[ ! $CSV_FILE ]]; then
  CSV_FILE="./sensors_by_city.csv"
  echo "using default: $CSV_FILE"
fi

export WORKPATH=$(readlink -f ./mirror)
export SENSORS_BY_CITY=$(readlink -f $CSV_FILE)

mkdir -p $WORKPATH

pushd $WORKPATH

if [ -f "$SENSORS_BY_CITY" ]; then
    echo "$SENSORS_BY_CITY found."
else
    echo "$SENSORS_BY_CITY not found. You need a $SENSORS_BY_CITY file."
    exit 1
fi

IFS=$','
SPLIT_SENSORS=$(echo $SENSORS | sed "s/,/ /g" | awk '{$1=$1};1')

host archive.sensor.community
sleep 1

echo start date: $START
echo end date: $END
echo sensor IDs: $SPLIT_SENSORS
echo csv file: $CSV_FILE


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

        # sensor_id,sensor_type,node,city
        # $split is now a bash array
        sensor_id=${split[0]}
        sensor_type=$(echo "${split[1]}" | sed 's/./\L&/g')
        chip_id=${split[2]}
        city=${split[3]}

        if ! ([[ -z "$SPLIT_SENSORS" ]] || [[ ${SPLIT_SENSORS[@]} =~ $sensor_id ]])
        then
          # sensor id not found
          continue
        fi
        echo $line

        file="${DATE}_${sensor_type}_sensor_${sensor_id}.csv"
        location=https://archive.sensor.community/$DATE/$file

        echo "Downloading $file"
        curl -O $location

        if [ ! -f "$file" ]; then
            echo "$file not found at $location"
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
