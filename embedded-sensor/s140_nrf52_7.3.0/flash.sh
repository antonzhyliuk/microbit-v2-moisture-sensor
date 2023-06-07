#!/bin/sh

probe-rs-cli erase --chip nrf52833
probe-rs-cli download --chip nrf52833 --format hex s140_nrf52_7.3.0_softdevice.hex
