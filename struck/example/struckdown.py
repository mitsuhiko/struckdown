import sys
import json
import functools


def read_event(line):
    return json.loads(line)


def dump_event(cmd):
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
