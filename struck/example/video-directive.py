import html
from struckdown import streamprocessor


def expand_video_directive(cmd):
    id = html.escape(cmd["front_matter"]["id"])
    width = int(cmd["front_matter"].get("width") or 640)
    height = int(cmd["front_matter"].get("height") or 360)
    return {
        "type": "raw_html",
        "html": (
            f"""<iframe type="text/html" width="{width}" height="{height}" src="https://www.youtube.com/embed/{id}?autoplay=1" frameborder=0></iframe>\n"""
        )
    }


@streamprocessor
def main(events):
    for event in events:
        cmd, location = event
        if cmd["type"] == "directive" and cmd["name"] == "youtube":
            yield expand_video_directive(cmd), location
        else:
            yield event


if __name__ == "__main__":
    main()