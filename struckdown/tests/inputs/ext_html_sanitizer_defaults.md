---
processors:
  - processor: html_sanitizer
---

# Hello World!

<div class="hello">

*Here is some Markdown*

</div>

<style>
  background: blue
</style>
<div style="background: red">
  <p>This shit is safe.
  <script>alert("This shit is unsafe");</script>
</div>

More stuff here.