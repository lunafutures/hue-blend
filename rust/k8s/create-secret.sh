#! /bin/bash

kubectl create secret generic rust-hue-schedule --from-file=../private/schedule.yml