#!/usr/bin/env bash
# set -x

cd hivetests/

sim="${1}"
limit="${2}"

run_hive() {
    hive --sim "${sim}" --sim.limit "${limit}" --sim.parallelism 4 --client reth 2>&1 | tee /tmp/log || true
}

check_log() {
    tail -n 1 /tmp/log | sed -r 's/\x1B\[[0-9;]*[mK]//g'
}

attempt=0
max_attempts=5

while [ $attempt -lt $max_attempts ]; do
    run_hive

    # Check if no tests were run. sed removes ansi colors
    if check_log | grep -q "suites=0"; then
<<<<<<< HEAD
        echo "no tests were run, retrying in 5 seconds"
        sleep 5
=======
        echo "no tests were run, retrying in 10 seconds"
        sleep 10
>>>>>>> c4b5f5e9c9a88783b2def3ab1cc880b8d41867e1
        attempt=$((attempt + 1))
        continue
    fi

    # Check the last line of the log for "finished", "tests failed", or "test failed"
    if check_log | grep -Eq "(finished|tests? failed)"; then
        exit 0
    else
        exit 1
    fi
done
<<<<<<< HEAD
exit 1
=======
exit 1
>>>>>>> c4b5f5e9c9a88783b2def3ab1cc880b8d41867e1
