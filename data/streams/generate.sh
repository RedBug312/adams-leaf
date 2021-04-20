#!/usr/bin/env bash

END_DEVICES=9

_tsn() {
    ends=$(shuf -i 0-$END_DEVICES -n 2)
    period=$(shuf -n 1 -e 100 200 250)
    echo - src: $(echo $ends | cut -d ' ' -f 1)
    echo ' ' dst: $(echo $ends | cut -d ' ' -f 2)
    echo ' ' size: "$(shuf -n 1 -i 5-15)00"
    echo ' ' period: $period
    echo ' ' deadline: $period
    echo ' ' offset: 0
}

_avb() {
    ends=$(shuf -i 0-$END_DEVICES -n 2)
    period=$(shuf -n 1 -e 100 200 250)
    echo - src: $(echo $ends | cut -d ' ' -f 1)
    echo ' ' dst: $(echo $ends | cut -d ' ' -f 2)
    echo ' ' size: 400
    echo ' ' period: $period
    echo ' ' deadline: $period
    echo ' ' class: $(shuf -n 1 -e A B)
}

echo name: UNKNOWN
echo
echo scale:
echo ' ' tsns: $1
echo ' ' avbs: $2
echo ' ' hyperperiod: 1000
echo ' ' end_devices: $((END_DEVICES + 1))
echo
echo tsns:
for _ in $(seq $1); do _tsn; done
echo
echo avbs:
for _ in $(seq $2); do _avb; done
