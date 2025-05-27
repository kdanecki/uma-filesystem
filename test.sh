#!/bin/bash

set -x

cat mp/foo
cat mp/XD/xd

mkdir mp/LOL
echo "bvibsfjcrsnw" > mp/LOL/lul
echo "abc" > mp/LOL/lul
echo "def" >> mp/LOL/lul
cat mp/LOL/lul

rm -r mp/LOL

ls -l mp
tree mp
