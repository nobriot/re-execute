#!/bin/sh
for i in $(seq 1 30); do
  echo "This is line $i - file $1"
  sleep 0.1
done
