import html
from struckdown import streamprocessor


def expand_video_directive(event):
    id = html.escape(event["front_matter"]["id"])
    width = int(event["front_matter"].get("width") or 640)
    height = int(event["front_matter"].get("height") or 360)
    return {
        "type": "raw_html",
        "html": (
            f"""<iframe type="text/html" width="{width}" height="{height}" src="https://www.youtube.com/embed/{id}?autoplay=1" frameborder=0></iframe>\n"""
        ),
        "location": event.get("location"),
    }


@streamprocessor
def main(events):
    for event in events:
        if event["type"] == "directive" and event["name"] == "youtube":
            yield expand_video_directive(event)
        else:
            yield event


if __name__ == "__main__":
    main()
