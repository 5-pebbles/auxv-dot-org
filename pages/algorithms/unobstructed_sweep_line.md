<head>
  <title>Sweep Line Algorithm | Auxv.org</title>
  <meta name="author" content="Owen Friedman">
  <meta name="description" content="This sweep line algorithm identifies all unique gaps between rectangular obstructions 📊...
">
</head>

# My Super Awesome Unobstructed Rectangle Sweep Line Algorithm 📊

Long ago, I was writing a web-automation framework for botting(🤖), and noticed that there was no way to find an unobstructed point on an element. I knew where the element was, but I couldn't click it via JavaScript for risk of detection.

<br/>

Action chains had been my go-to solution, however, if the element is partially obstructed you run the risk of clicking the wrong thing. My only savior was the `elementsFromPoint` JavaScript API.

<br/>

This led to the invention of the `SmartClick TM` system:

1. Pick a random point on your target element (weighted towards the center for realism).
2. Try `elementsFromPoint`, if there are any obstructions, add them to a list.
3. Find the largest gap and repeat. <br/>
   (until the first clickable element is your target, or you run out of gaps)

However, I could not find a fast algorithm to locate these gaps, so I did it myself... Enjoy!

<br/>
<details>
<summary><b>Table of Contents:</b></summary>

- [My Super Awesome Unobstructed Rectangle Sweep Line Algorithm 📊](#my-supper-awesome-unobstructed-rectangle-sweep-line-algorithm)
- [Section 1: Locating Important Lines 🔭](#section-1-locating-important-lines)
- [Section 2: Finding Gaps Between Obstructions 📏](#section-2-finding-gaps-between-obstructions)
- [Section 3: Rectangle Identification 🧱](#section-3-rectangle-identification)
- [Section 4: Finalization 🏁](#section-4-finalization)
- [Section 5: Implementation 🔩](#section-5-implementation)

</details>


## Section 1: Locating Important Lines 🔭

Traversing every point on a parent rectangle (target element) would be slow and wasteful. If all rectangles are running in parallel, we need only check the start and end of each obstruction.
<br/>

### Opening Lines

These appear one unit after the end of each obstruction. This is the only place where a new rectangle may begin.

I also use an `opening line` on the first unit of the parent rectangle, because it is easier than manually implementing that case.

### Closing Lines

These appear on the first unit of each obstruction. This is the only place where a rectangle may end or be subdivided.

<br/>

**Take this example:**

![image](/algorithms/diagrams/unobstructed_sweep_line-1.svg)

**The algorithm will draw four lines:**

- A `opening line` at the start of the parent rectangle.
- A `closing line` at the start of the first obstruction (same location as the parent's opening line).
- A `opening line` at the end of the first obstruction.
- A `closing line` at the start of the second obstruction.

_**NOTE:**_ If a line starts before or ends after the parent rectangle, such as the `closing line` of the last obstruction, it is discarded.

![image](/algorithms/diagrams/unobstructed_sweep_line-2.svg)


<details>
<summary><b>Struct Definition:</b></summary>

```rs
/// A line we need to check for gaps.
struct Line<T> {
    x: T,
    opens: bool,
}
```

</details>


## Section 2: Finding Gaps Between Obstructions 📏

The algorithm can identify all gaps on each line, by filtering out obstructions that don't intersect and then sorting them by the top edge. As it iterates downward we save a pointer to the bottom of the last obstruction, this starts initialized to the top of the parent rectangle.

<br/>

**On each obstruction:**

1. It checks if the pointer to the bottom of the last obstruction is above the current obstruction's top edge; if it is, there is a gap between them.
2. Then it updates the pointer to the minimum of its current value and this obstruction's bottom. This is required to handle obstructions that overlap.

<br/>

**Example of overlapping obstructions:**

![image](/algorithms/diagrams/unobstructed_sweep_line-3.svg)

The outer obstruction is processed first; if the pointer was just updated to the inner obstruction, a false gap would be found.


<details>
<summary><b>Struct Definition:</b></summary>

```rs
/// A gap between two obstructions.
struct Gap<T> {
    top: T,
    bottom: T,
}
```

</details>


## Section 3: Rectangle Identification 🧱

As the algorithm process the gaps from left to right, it maintains a list of active rectangles and a list of completed rectangles.

### Opening Lines

When it encounters a gap on an `opening line` without an already active rectangle. It adds a new one starting at the current line with the gap's top and bottom.

![image](/algorithms/diagrams/unobstructed_sweep_line-4.svg)

### Closing Lines

When it encounters a `closing line` the algorithm checks if each active rectangle fits within one of the gaps. If it does, the rectangle continues uninterrupted, otherwise:
- It is added to the completed rectangles list, ending one unit before the current line started.
  - If the closed rectangle was only partially obstructed, it is also subdivided into the gaps contained within; the new active rectangles have the same start position as the original.

![image](/algorithms/diagrams/unobstructed_sweep_line-5.svg)

<details>
<summary><b>Struct Definition:</b></summary>

```rs
/// A rectangle that has not been obstructed yet.
#[derive(Clone)]
struct UnfinishedRect<T> {
    left: T, // Start
    top: T,
    bottom: T,
}
```

</details>


## Section 4: Finalization 🏁

After processing all lines, any remaining active rectangles are added to the completed rectangles list. Their right edges extend to the parent rectangle's boundary.

**In our example, this leaves us with four rectangles:**

![image](/algorithms/diagrams/unobstructed_sweep_line-6.svg)

## Section 5: Implementation 🔩

**Here is the algorithm as I implemented it in the [rect-lib](https://github.com/5-pebbles/rect-lib) crate:**

<div id="raw-content"></div>
<script>
  fetch('https://raw.githubusercontent.com/5-pebbles/rect-lib/refs/heads/main/src/unobstructed_sweep_line.rs')
    .then(response => response.text())
    .then(text => {
      const escapedText = text
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;');
      document.getElementById('raw-content').innerHTML = `<pre><code class="language-rs">${escapedText}</code></pre>`;
      hljs.highlightAll();
    })
    .catch(error => {
      document.getElementById('raw-content').innerHTML = 'Error loading content';
    });
</script>

> If there is anything that can be improved, please contact [me](/about) or create an [issue](https://github.com/5-pebbles/rect-lib/issues).

