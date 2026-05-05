#!/usr/bin/env bash

sleep_ms=$((RANDOM % 5000000))
return_code=$((RANDOM % 4))

echo "sleep: ${sleep_ms} ms - return: $return_code"

sleep "$((sleep_ms / 1000)).$(printf '%03d' $((sleep_ms % 1000000)))"
exit $return_code
