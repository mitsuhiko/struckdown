Example directive with nested parsing:

````{video} inline-argument
---
url: https://www.youtube.com/watch?v=dQw4w9WgXcQ
title: Rick Roll
---

Raw body content that goes with the directive.

```{nested} nested-inline-argument
This is an inline directive without front-matter arguments.
```
````

Invalid front matter:

```{invalid}
---
[[[[
---
stuff
```
