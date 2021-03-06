#!/usr/bin/env sh

grep '#2 computing time' $@ \
    | cut -d ' ' -f 5
