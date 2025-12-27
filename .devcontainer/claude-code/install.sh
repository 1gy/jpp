#!/bin/bash

mkdir -p ${CLAUDE_CONFIG_DIR}
chown -R ${_REMOTE_USER}:${_REMOTE_USER} ${CLAUDE_CONFIG_DIR}

su - ${_REMOTE_USER} -c "curl -fsSL https://claude.ai/install.sh | bash -s latest"
