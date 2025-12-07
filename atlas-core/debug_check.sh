#!/bin/bash
echo "--- LS ---" > debug_output.txt
ls -F >> debug_output.txt
echo "--- PGREP ---" >> debug_output.txt
pgrep -a atlas-core >> debug_output.txt
echo "--- LOGS DIR ---" >> debug_output.txt
ls -F logs/ >> debug_output.txt
echo "--- NODE1 LOG ---" >> debug_output.txt
cat logs/consensus-node1.log >> debug_output.txt
echo "--- DONE ---" >> debug_output.txt
