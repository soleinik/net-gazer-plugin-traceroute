#!/bin/bash

# to install flatc
# sudo apt install -y flatbuffers-compiler
OUT_FILE=src/traceroute.rs

flatc --rust \
 -o src \
 traceroute.fbs 


# planus rust -o $OUT_FILE traceroute.fbs