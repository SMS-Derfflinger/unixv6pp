#!/bin/sh

while read -r filename; do
        if ! file src/kernel/main.cpp | grep -q CRLF; then
                continue
        fi

        if ! cat "$filename" | tr -d "\r" > "$filename.new"; then
                echo "$filename failed"
                continue
        fi

        mv "$filename.new" "$filename"
done <<< "$(find src -type f \( -name '*.cpp' -or -name '*.h' \))"
