---
description: Example markdown file with a directive
---

# Hello YouTube

This is an example that has a _custom directive_ which can be expanded with the
`video-directive.py` file:

```
struck parse example/video.md | python3 example/video-directive.py | struck render-stream
```

And this is what is being processed:

```{youtube}
---
id: Gzj723LkRJY
width: 640
height: 360
---
```

That's it!
