#!/bin/bash

set -x

echo abc > mp/foo
mkdir mp/XD
echo loool > mp/XD/xd

cat mp/foo
cat mp/XD/xd

mkdir mp/LOL
echo "bvibsfjcrsnw" > mp/LOL/lul
echo "abc" > mp/LOL/lul
echo "def" >> mp/LOL/lul
cat mp/LOL/lul


cp small.html mp/small.html
cp index.html mp/index.html

ls -l mp
tree mp

rm mp/foo
rm -r mp/XD
rm -r mp/LOL
rm mp/small.html
rm mp/index.html
