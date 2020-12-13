import sys
import json
import functools


def read_event(line):
    rv = json.loads(line)
    if isinstance(rv, list):
        cmd, location = rv
        return cmd, location
    return rv, None


def dump_event(cmd, location=None):
    if location is not None:
        arg = [cmd, location]
    else:
        arg = cmd
    return json.dumps(cmd).strip()


def iter_events(stream=None):
    if stream is None:
        stream = sys.stdin
    for line in stream:
        yield read_event(line)


def streamprocessor(f):
    @functools.wraps(f)
    def wrapper():
        for event in f(iter_events()):
            print(dump_event(event))
    return wrapper