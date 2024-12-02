#!/bin/bash

clientsno=${CLIENTSNO:-8}

# Generate random postfix for session name
postfix=$(shuf -i 1000000000-9999999999 -n 1)

# Create session name
session_name="demo-tmux-$postfix"

set -x

# Open Tmux
tmux new-session -d -s $session_name 'cargo run --example actix_web_and_indicatif_v03'

# Split vertically into panes
for ((i=1; i<=clientsno; i++)); do
  tmux split-window -v -t $session_name
  tmux select-layout -t $session_name main-vertical
done

# Run curl command in each pane with random sleep
for ((i=1; i<=(clientsno); i++)); do
  tmux send-keys -t $session_name.$i "while true ; do curl 'http://localhost:11555' ; sleep $((RANDOM % 3)) ; done" C-m
done

sleep 1
# Attach to the tmux session
tmux attach -t $session_name
