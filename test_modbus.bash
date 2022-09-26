#!/usr/bin/env bash
echo "writing to modbus"
./target/debug/modbus 127.0.0.1:5502 write-register --address 420 --value 2
echo ""
echo "reading from modbus"
./target/debug/modbus 127.0.0.1:5502 read-register --address 420 --count 1 --kind holding -p hex -u 1
echo ""
echo "tcp error"
./target/debug/modbus 127.0.0.1:5501 write-register --address 420 --value 2
echo ""
echo "invalid register"
./target/debug/modbus 127.0.0.1:5501 write-register --address 650000 --value 2
echo ""
echo "invalid register kind"
./target/debug/modbus 127.0.0.1:5502 read-register --address 420 --count 1 --kind potato -p crabs -u 1