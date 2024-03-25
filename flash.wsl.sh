#!/bin/bash

source ./.env

# run on windows host (=v1.5.0)
espflash.exe ${SERIAL_PORT} $(wslpath -w "${1}")