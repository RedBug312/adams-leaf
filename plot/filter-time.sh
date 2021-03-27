#!/usr/bin/env sh

grep '#2 elapsed time' $@ \
    | cut -d ' ' -f 5
