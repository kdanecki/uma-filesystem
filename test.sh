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

cp small.html mp/small.html
cp index.html mp/index.html

ls -l mp
tree mp

rm mp/small.html
rm mp/index.html
