#!/usr/bin/env sh

# Used in Makefile, extract average compute time from logs
# $ utils/stat.sh plot/log/aco-mid-{1,2,3,4,5,6,7}-3.log

sed -n '/compute time/s|[^0-9]||gp' $@ \
    | pr -$# -t \
