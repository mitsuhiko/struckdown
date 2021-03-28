from struckdown import streamprocessor


@streamprocessor
def main(events):
    for event in events:
        location = event.get("location")
        if event["type"] == "start_tag" and location is not None:
            attrs = event.setdefault("attrs", {})
            custom = attrs.setdefault("custom", {})
            custom["data-line"] = str(location["line"])
            custom["data-column"] = str(location["column"])
        yield event


if __name__ == "__main__":
    main()
