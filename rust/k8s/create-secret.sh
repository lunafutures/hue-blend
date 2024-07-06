#! /bin/bash

kubectl delete secret rust-hue-schedule
kubectl create secret generic rust-hue-schedule --from-file=../private/schedule.yml