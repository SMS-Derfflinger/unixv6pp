#!/bin/bash

funcname="$1"
shift

list="$(git grep "u_$funcname" 'src/*.cpp' 'src/*.h' | awk '{print $1}' | tr -d : | sort -u | paste -s -)"
echo "file list to change: $list"

git grep 'u_'"$funcname"

read yesno
if [ "$yesno" != y ]; then
        exit 1
fi

LC_ALL=C sed -i -E 's@u\.u_'"$funcname"'@User_get_'"$funcname"'()@g' $list

git grep 'u_'"$funcname"
