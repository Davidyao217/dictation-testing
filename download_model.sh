#!/bin/bash

MODEL_DIR="$HOME/.dictation/models"
MODEL_NAME="ggml-base.en.bin"
MODEL_URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin"

mkdir -p "$MODEL_DIR"

if [ -f "$MODEL_DIR/$MODEL_NAME" ]; then
    echo "Model already exists at $MODEL_DIR/$MODEL_NAME"
    exit 0
fi

echo "Downloading Whisper model (base.en - 148MB)..."
curl -L -o "$MODEL_DIR/$MODEL_NAME" "$MODEL_URL"

if [ $? -eq 0 ]; then
    echo "Model downloaded successfully to $MODEL_DIR/$MODEL_NAME"
else
    echo "Failed to download model"
    exit 1
fi
