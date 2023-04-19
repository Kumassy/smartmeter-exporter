#!/bin/bash
source .env
cross build --target armv7-unknown-linux-gnueabihf && \
rsync -avzP ./target/armv7-unknown-linux-gnueabihf/debug/smartmeter-exporter pi:/home/pi/smartmeter-exporter/ && \
ssh pi "RUST_LOG=debug /home/pi/smartmeter-exporter/smartmeter-exporter"