#!/bin/bash

set -x

echo abc > mp/foo
mkdir mp/XD
echo loool > mp/XD/xd

cat mp/foo
cat mp/XD/xd

mkdir mp/LOL
echo "blablablabla" > mp/LOL/lol
echo "abc" > mp/LOL/lol
echo "def" >> mp/LOL/lol
cat mp/LOL/lol


cp oxidizedFS mp/oxidizedFS
cp target/debug/libfs_rust.so mp/libfs_rust.so

ls -l mp
tree mp

rm mp/foo
rm -r mp/XD
rm -r mp/LOL
rm mp/oxidizedFS
rm mp/libfs_rust.so
