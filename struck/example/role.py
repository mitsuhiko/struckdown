import sys
from struckdown import streamprocessor


def expand_link_role(cmd):
    target = cmd['text']
    return [
        {
            "type": "start_tag",
            "tag": "link",
            "attrs": {
                "target": "https://example.com/api/%s'" % target,
                "class": "api-link",
            },
        },
        {
            "type": "text",
            "text": target,
        },
        {
            "type": "end_tag",
            "tag": "link",
        },
    ]


@streamprocessor
def main(events):
    for event in events:
        cmd, location = event
        if cmd['type'] == 'interpreted_text' and cmd['role'] == 'api':
            for event in expand_link_role(cmd):
                yield event, location
        else:
            yield event


if __name__ == "__main__":
    main()
