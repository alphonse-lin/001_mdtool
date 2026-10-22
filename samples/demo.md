# Project Notes

A quick tour of what **mdtool** renders — *instantly*, right from Explorer.

## Progress

- [x] Fast startup with WebView2
- [x] Live preview while editing
- [ ] World domination

| Feature | Supported | Notes |
|:--|:--:|:--|
| Tables, task lists, footnotes | ✅ | GitHub-flavored |
| Syntax highlighting | ✅ | 36 languages |
| Math | ✅ | KaTeX |

## Code

```rust
fn main() {
    let greeting = "Hello, Markdown!";
    println!("{greeting}");
}
```

## Math

The Gaussian integral $\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}$, and Euler's identity:

$$
e^{i\pi} + 1 = 0
$$

> Good tools get out of your way.[^1]

[^1]: Footnotes work too.
