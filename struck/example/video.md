---
description: Example markdown file with a directive
---

# Hello YouTube

This is an example that has a _custom directive_ which can be expanded with the
`video-directive.py` file:

```
struck parse example/video.md | struck process example/process.yml | struck render
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
