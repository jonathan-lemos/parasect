#!/bin/sh

sleep $((RANDOM % 5))

if [ $1 -le $2 ]; then
  exit 0
else
  exit 1
fi
