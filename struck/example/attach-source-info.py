from struckdown import streamprocessor


@streamprocessor
def main(events):
    for cmd, location in events:
        if cmd["type"] == "start_tag" and location is not None:
            attrs = cmd.setdefault("attrs", {})
            custom = attrs.setdefault("custom", {})
            custom["data-line"] = str(location["line"])
            custom["data-column"] = str(location["column"])
        yield cmd, location


if __name__ == "__main__":
    main()