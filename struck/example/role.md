---
description: Example markdown file with a role that gets expanded.
---

# Hello Roles

This shows how a role can be expanded.

## How to Run

```
struck parse example/role.md | struck process example/role.yml | struck render
```

## Output

And this is what is being processed:

Hello {api}`foo/bar`.

That's it!
