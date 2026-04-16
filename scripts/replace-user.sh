#!/bin/sh

funcname="$1"
shift

LC_ALL=C sed -i -E 's@u\.u_'"$funcname"'@User_get_'"$funcname"'()@g' "$@"
