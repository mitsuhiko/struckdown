import json
import sys
import html

"""
{
    "type": "directive",
    "name": "youtube",
    "argument": "inline-argument",
    "front_matter": {
        "url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
        "title": "Rick Roll"
    },
}
"""

def read_cmd(line):
    rv = json.loads(line)
    if isinstance(rv, list):
        return rv
    return [rv, None]

def dump_cmd(cmd, loc=None):
    if loc is not None:
        arg = [cmd, loc]
    else:
        arg = cmd
    return json.dumps(cmd).strip()


def expand_video_directive(cmd):
    id = html.escape(cmd["front_matter"]["id"])
    width = int(cmd["front_matter"].get("width") or 640)
    height = int(cmd["front_matter"].get("height") or 360)
    return [{
        "type": "raw_html",
        "html": (
            f"""<iframe type="text/html" width="{width}" height="{height}" src="https://www.youtube.com/embed/{id}?autoplay=1" frameborder=0></iframe>\n"""
        )
    }]


for line in sys.stdin:
    cmd, loc = read_cmd(line)
    if cmd["type"] == "directive" and cmd["name"] == "youtube":
        for new_cmd in expand_video_directive(cmd):
            print(dump_cmd(new_cmd))
        continue
    print(dump_cmd(cmd, line))
