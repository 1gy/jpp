#!/bin/bash

su - ${_REMOTE_USER} -c "curl --proto '=https' --tlsv1.2 -sSf https://just.systems/install.sh | bash -s -- --to ~/.local/bin"
