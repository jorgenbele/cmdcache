# cmdcache

Simple command line utility to cache command line programs.

## Why
Shell scripts can often be executed many times in rapid succession. Sometimes some part of the script needs to do an expensive operation (such as database access, network communication, etc) but we don't need the result to be updated every time. cmdcache let you easily cache results of invokations without manually dealing with caching in the script.

## How it works
cmdcache saves both the stdout and stderr output of a command every time it is executed. It checks whether the time since the last execution is less than `--cache-seconds`, and uses the existing output if it is too recent. 

Caching of database query 
```rust
$ cmdcache -c 3600 psql < costly_db_query.sql
```
