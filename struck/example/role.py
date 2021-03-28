import sys
from struckdown import streamprocessor


def expand_link_role(event):
    target = event['text']
    location = event.get("location")
    return [
        {
            "type": "start_tag",
            "tag": "link",
            "attrs": {
                "target": "https://example.com/api/%s'" % target,
                "class": "api-link",
            },
            "location": location,
        },
        {
            "type": "text",
            "text": target,
            "location": location,
        },
        {
            "type": "end_tag",
            "tag": "link",
            "location": location,
        },
    ]


@streamprocessor
def main(events):
    for event in events:
        if event['type'] == 'interpreted_text' and event['role'] == 'api':
            for event in expand_link_role(event):
                yield event
        else:
            yield event


if __name__ == "__main__":
    main()
