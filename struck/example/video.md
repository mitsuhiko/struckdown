---
description: Example markdown file with a directive
---

# Hello YouTube

This is an example that has a _custom directive_ which can be expanded with the
`video-directive.py` file.

```{toc}
```

## How to Run

```
struck parse example/video.md | struck process example/video.yml | struck render
```

## Output (_When run_)

And this is what is being processed:

```{youtube}
---
id: Gzj723LkRJY
width: 640
height: 360
---
```

That's it!
