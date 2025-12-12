#!/bin/bash

echo "🧹 Cleaning node data directories..."

rm -rf example/node*/data
rm -rf audits

echo "✨ Done! All data and audits cleared."
