#!/bin/bash

source ./.env

# run on windows host (=v1.5.0)
espflash.exe \
    flash \
    -p ${SERIAL_PORT} \
    --baud=115200 \
    $(wslpath -w "${1}")