#!/usr/bin/env bash

_args() {
    basename -s .log $@ \
        | awk -F - 'BEGIN {OFS="\t"} {print $1"-"$4, $2, $3*10, $5}'
}

_time() {
    tail -n 1 $@ \
        | grep '#2 elapsed time' \
        | cut -d ' ' -f 5
}

paste <(_args $@) <(_time $@)
