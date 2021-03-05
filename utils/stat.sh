#!/usr/bin/env sh

# Used in Makefile, extract average compute time from logs
# $ utils/stat.sh plot/log/aco-mid-{1,2,3,4,5,6,7}-3.log

grep '#2 computing time' $@ \
    | cut -d ' ' -f 5 \
    | pr -$# -t \
    | datamash -W mean 1-$#; \
