#!/bin/bash

ZIP_NAME="fiducial.zip"
TARGET_DIR="./"

if [[ -d "$TARGET_DIR/src" && -d "$TARGET_DIR/proto" && -f "$TARGET_DIR/Cargo.toml" ]]; then
    cd "$TARGET_DIR"
    zip -r "$ZIP_NAME" src proto Cargo.toml
    echo "Arquivos zipados com sucesso em $ZIP_NAME"
else
    echo "Erro: Um ou mais arquivos/diretórios não encontrados em $TARGET_DIR"
    exit 1
fi
